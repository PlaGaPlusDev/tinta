//! Overlays: quick switcher (Ctrl+O) y paleta de comandos (Ctrl+P).
//! Mecánica compartida: entrada de texto con filtro fuzzy + lista navegable.

use egui::{Align2, Area, Context, Frame, Key, Margin, RichText, Ui};

use crate::commands::{all_commands, Command};
use crate::fuzzy::filter_ranked;
use crate::vault::Vault;

/// Elección del overlay activo.
pub enum OverlayPick {
    Note(usize),
    Command(Command),
    None,
}

#[derive(Default)]
pub struct Overlays {
    pub switcher_open: bool,
    pub palette_open: bool,
    pub query: String,
    pub selected: usize,
    needs_focus: bool,
}

impl Overlays {
    pub fn open_switcher(&mut self) {
        self.switcher_open = true;
        self.palette_open = false;
        self.query.clear();
        self.selected = 0;
        self.needs_focus = true;
    }

    pub fn open_palette(&mut self) {
        self.palette_open = true;
        self.switcher_open = false;
        self.query.clear();
        self.selected = 0;
        self.needs_focus = true;
    }

    pub fn close(&mut self) {
        self.switcher_open = false;
        self.palette_open = false;
        self.query.clear();
        self.selected = 0;
    }

    pub fn any_open(&self) -> bool {
        self.switcher_open || self.palette_open
    }

    /// Quick switcher: filtra `vault.notes`, devuelve el índice elegido.
    pub fn draw_switcher(&mut self, ctx: &Context, vault: &Vault) -> OverlayPick {
        let items: Vec<(usize, String)> = vault
            .notes
            .iter()
            .enumerate()
            .map(|(i, n)| (i, n.rel_path.clone()))
            .collect();
        match self.draw(ctx, "Ir a nota…", &items) {
            OverlayResult::Picked(index) => OverlayPick::Note(index),
            OverlayResult::StillOpen => OverlayPick::None,
        }
    }

    /// Paleta de comandos.
    pub fn draw_palette(&mut self, ctx: &Context) -> OverlayPick {
        let commands = all_commands();
        let items: Vec<(usize, String)> = commands
            .iter()
            .enumerate()
            .map(|(i, (_, label))| (i, label.clone()))
            .collect();
        match self.draw(ctx, "Ejecutar comando…", &items) {
            OverlayResult::Picked(i) => OverlayPick::Command(commands[i].0),
            OverlayResult::StillOpen => OverlayPick::None,
        }
    }

    /// Núcleo compartido. Devuelve el índice del elemento elegido, si lo hay.
    fn draw(
        &mut self,
        ctx: &Context,
        title: &str,
        items: &[(usize, String)],
    ) -> OverlayResult {
        if self.any_open() {
            // Navegación con teclado al nivel de app.
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.close();
                return OverlayResult::StillOpen;
            }
            if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
                self.selected += 1;
            }
            if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
                self.selected = self.selected.saturating_sub(1);
            }
        }

        let id = egui::Id::new(title);
        let query = self.query.clone();
        let ranked = filter_ranked(&query, items);
        if self.selected >= ranked.len() {
            self.selected = ranked.len().saturating_sub(1);
        }
        let selected = self.selected;

        let mut clicked: Option<usize> = None;
        let mut wants_quit_focus: bool = false;

        Area::new(id)
            .anchor(Align2::CENTER_TOP, [0.0, 60.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                Frame::default()
                    .inner_margin(Margin::same(12))
                    .corner_radius(egui::CornerRadius::same(8))
                    .fill(ui.style().visuals.extreme_bg_color)
                    .show(ui, |ui| {
                        ui.set_min_width(420.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(title).weak().small());
                            ui.add_space(4.0);
                            let edit = egui::TextEdit::singleline(&mut self.query)
                                .hint_text("Escribe para filtrar…")
                                .desired_width(420.0);
                            let resp = ui.add(edit);
                            if self.needs_focus {
                                resp.request_focus();
                                self.needs_focus = false;
                            }
                            if ctx.input(|i| i.key_pressed(Key::Enter)) {
                                wants_quit_focus = true;
                            }
                            ui.add_space(6.0);
                            egui::ScrollArea::vertical().max_height(320.0).show(
                                ui,
                                |ui| {
                                    for (row, (_, item_id)) in ranked.iter().enumerate() {
                                        let (_, text) =
                                            items.iter().find(|(v, _)| *v == *item_id).unwrap();
                                        if ui
                                            .add(
                                                egui::Button::selectable(
                                                    selected == row,
                                                    RichText::new(text.clone()),
                                                )
                                                .min_size(egui::vec2(400.0, 20.0)),
                                            )
                                            .clicked()
                                        {
                                            clicked = Some(*item_id);
                                        }
                                    }
                                },
                            );
                        });
                    });
            });

        if wants_quit_focus {
            let picked = ranked.get(selected).map(|(_, id)| *id);
            self.close();
            if let Some(id) = picked {
                return OverlayResult::Picked(id);
            }
        }
        if let Some(id) = clicked {
            self.close();
            return OverlayResult::Picked(id);
        }
        OverlayResult::StillOpen
    }
}

enum OverlayResult {
    Picked(usize),
    StillOpen,
}
