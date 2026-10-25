//! Overlays: quick switcher (Ctrl+O), paleta de comandos (Ctrl+P)
//! y búsqueda en vault (Ctrl+Shift+F). Mecánica común: filtrar + listar.

use egui::{Align2, Area, Context, CornerRadius, Frame, Key, Margin, RichText};

use crate::commands::Command;
use crate::fuzzy::filter_ranked;
use crate::search::SearchHit;
use crate::vault::Vault;

/// Elección del overlay activo.
pub enum OverlayPick {
    Note(usize),
    Command(Command),
    Search(usize),
    None,
}

#[derive(Default)]
pub struct Overlays {
    pub switcher_open: bool,
    pub palette_open: bool,
    pub search_open: bool,
    pub query: String,
    pub selected: usize,
    needs_focus: bool,
}

impl Overlays {
    pub fn open_switcher(&mut self) {
        self.reset_focus(Option::<String>::None);
        self.switcher_open = true;
    }

    pub fn open_palette(&mut self) {
        self.reset_focus(Option::<String>::None);
        self.palette_open = true;
    }

    /// Abre búsqueda textual, con query inicial opcional (ej. un tag).
    pub fn open_search(&mut self, initial: Option<String>) {
        self.reset_focus(initial);
        self.search_open = true;
    }

    /// Reinicia estado y enfoca el campo de texto del próximo frame.
    fn reset_focus(&mut self, initial: Option<String>) {
        self.switcher_open = false;
        self.palette_open = false;
        self.search_open = false;
        self.query = initial.unwrap_or_default();
        self.selected = 0;
        self.needs_focus = true;
    }

    pub fn close(&mut self) {
        self.switcher_open = false;
        self.palette_open = false;
        self.search_open = false;
        self.query.clear();
        self.selected = 0;
    }

    pub fn any_open(&self) -> bool {
        self.switcher_open || self.palette_open || self.search_open
    }

    /// Quick switcher: filtra las notas del vault (rel_path / título).
    pub fn draw_switcher(&mut self, ctx: &Context, vault: &Vault) -> OverlayPick {
        let items: Vec<(usize, String)> = vault
            .notes
            .iter()
            .enumerate()
            .map(|(i, n)| (i, n.rel_path.clone()))
            .collect();
        match self.core_draw(ctx, "Ir a nota…", &items) {
            Some(id) => OverlayPick::Note(id),
            None => OverlayPick::None,
        }
    }

    /// Paleta de comandos.
    pub fn draw_palette(&mut self, ctx: &Context) -> OverlayPick {
        let commands = Command::all();
        let items: Vec<(usize, String)> = commands
            .iter()
            .enumerate()
            .map(|(i, (_, label))| (i, label.clone()))
            .collect();
        match self.core_draw(ctx, "Ejecutar comando…", &items) {
            Some(id) => OverlayPick::Command(commands[id].0),
            None => OverlayPick::None,
        }
    }

    /// Búsqueda textual en el vault: resultados en vivo con contexto.
    pub fn draw_search(&mut self, ctx: &Context, hits: &[SearchHit]) -> OverlayPick {
        let items: Vec<(usize, String)> = hits
            .iter()
            .enumerate()
            .take(80)
            .map(|(i, h)| (i, format!("{}:{}  —  {}", h.rel_path, h.line_no, h.line)))
            .collect();
        match self.core_draw(ctx, "Buscar en vault…", &items) {
            Some(id) => OverlayPick::Search(id),
            None => OverlayPick::None,
        }
    }

    /// Núcleo compartido: entrada, filtro fuzzy, teclado + click.
    fn core_draw(
        &mut self,
        ctx: &Context,
        title: &str,
        items: &[(usize, String)],
    ) -> Option<usize> {
        // Teclado de primer nivel.
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.close();
            return None;
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
            self.selected += 1;
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
            self.selected = self.selected.saturating_sub(1);
        }

        let query = self.query.clone();
        let ranked = filter_ranked(&query, items);
        self.selected = self.selected.min(ranked.len().saturating_sub(1));
        let selected = self.selected;

        let mut clicked: Option<usize> = None;
        let mut press_enter = false;

        Area::new(egui::Id::new(title))
            .anchor(Align2::CENTER_TOP, [0.0, 50.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                Frame::default()
                    .inner_margin(Margin::same(12))
                    .corner_radius(CornerRadius::same(8))
                    .fill(ui.style().visuals.extreme_bg_color)
                    .show(ui, |ui| {
                        ui.set_min_width(460.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(title).weak().small());
                            ui.add_space(4.0);
                            let edit = egui::TextEdit::singleline(&mut self.query)
                                .hint_text("Escribe para filtrar…")
                                .desired_width(440.0);
                            let resp = ui.add(edit);
                            if self.needs_focus {
                                resp.request_focus();
                                self.needs_focus = false;
                            }
                            if ctx.input(|i| i.key_pressed(Key::Enter)) {
                                press_enter = true;
                            }
                            ui.add_space(6.0);
                            egui::ScrollArea::vertical().max_height(340.0).show(
                                ui,
                                |ui| {
                                    for (row, (_, item_id)) in ranked.iter().enumerate() {
                                        let (_, text) = items
                                            .iter()
                                            .find(|(v, _)| *v == *item_id)
                                            .unwrap();
                                        let label =
                                            RichText::new(text.clone()).strong(row == selected);
                                        if ui
                                            .add(
                                                egui::Button::selectable(selected == row, label)
                                                    .min_size(egui::vec2(440.0, 20.0)),
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

        if press_enter {
            if let Some(item) = ranked.get(selected) {
                let id = item.1;
                self.close();
                return Some(id);
            }
        }
        if let Some(id) = clicked {
            self.close();
            return Some(id);
        }
        None
    }
}
