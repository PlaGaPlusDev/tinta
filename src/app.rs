//! Aplicación principal: paneles, editor, backlinks y overlays.

use egui::{CentralPanel, Color32, Context, Key, Modifiers, RichText, SidePanel, TopBottomPanel, Ui};
use rfd::FileDialog;

use crate::commands::Command;
use crate::editor::render_markdown;
use crate::index::{build_backlinks, Backlinks};
use crate::switcher::{OverlayPick, Overlays};
use crate::vault::{Note, Vault};

pub struct TintaApp {
    vault: Vault,
    current: Option<Note>,
    text: String,
    saved_text: String,
    backlinks: Backlinks,
    dark: bool,
    reading: bool,
    show_backlinks: bool,
    overlays: Overlays,
    sidebar_selected: usize,
    pending_jump: Option<String>,
}

impl TintaApp {
    pub fn new(vault_path: Option<String>) -> Self {
        let vault = vault_path.map_or_else(Vault::default, |p| Vault::open(&p));
        let mut app = Self {
            vault,
            current: None,
            text: String::new(),
            saved_text: String::new(),
            backlinks: Backlinks::default(),
            dark: true,
            reading: false,
            show_backlinks: true,
            overlays: Overlays::default(),
            sidebar_selected: 0,
            pending_jump: None,
        };
        let has_notes = !app.vault.notes.is_empty();
        if has_notes {
            app.open_note(0);
            app.backlinks = build_backlinks(&app.vault);
        }
        app
    }

    /// Abre la nota por índice del vault (cacheado tras escaneo).
    fn open_note(&mut self, index: usize) {
        let Some(note) = self.vault.notes.get(index).cloned() else { return };
        self.text = std::fs::read_to_string(&note.path).unwrap_or_default();
        self.saved_text = self.text.clone();
        self.current = Some(note);
        self.sidebar_selected = index;
    }

    fn open_note_rel(&mut self, rel_path: &str) {
        if let Some(i) = self.vault.notes.iter().position(|n| n.rel_path == rel_path) {
            self.open_note(i);
        }
    }

    fn save_current(&mut self) {
        if let Some(note) = &self.current {
            let _ = std::fs::write(&note.path, &self.text);
            self.saved_text = self.text.clone();
            self.backlinks = build_backlinks(&self.vault);
        }
    }

    fn open_vault_dialog(&mut self) {
        if let Some(folder) = FileDialog::new().pick_folder() {
            self.vault = Vault::open(&folder.to_string_lossy());
            let has_notes = !self.vault.notes.is_empty();
            self.current = None;
            self.text.clear();
            self.saved_text.clear();
            if has_notes {
                self.open_note(0);
                self.backlinks = build_backlinks(&self.vault);
            }
        }
    }

    fn new_note(&mut self) {
        let mut path = self.vault.root.join("nueva-nota.md");
        let mut n = 1;
        while path.exists() {
            path = self.vault.root.join(format!("nueva-nota-{n}.md"));
            n += 1;
        }
        let _ = std::fs::write(&path, "");
        self.vault.scan();
        if let Ok(rel) = path.strip_prefix(&self.vault.root) {
            self.open_note_rel(&rel.to_string_lossy().replace('\\', "/"));
        }
        self.backlinks = build_backlinks(&self.vault);
    }

    fn reindex(&mut self) {
        self.vault.scan();
        self.backlinks = build_backlinks(&self.vault);
    }

    fn run_command(&mut self, command: Command) {
        match command {
            Command::TogglePreview => self.reading = !self.reading,
            Command::ToggleBacklinks => self.show_backlinks = !self.show_backlinks,
            Command::RescanVault => self.reindex(),
            Command::OpenVault => self.open_vault_dialog(),
            Command::NewNote => self.new_note(),
            Command::ToggleTheme => self.dark = !self.dark,
            Command::Quit => std::process::exit(0),
        }
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        if self.overlays.any_open() {
            return; // Esc/Enter/↑/↓ los gestiona el overlay.
        }
        let ctrl = ctx.input(|i| i.modifiers.contains(Modifiers::CTRL));
        let some_ctrl = ctrl;
        let up = ctx.input(|i| i.key_pressed(Key::O));
        let pp = ctx.input(|i| i.key_pressed(Key::P));
        let ss = ctx.input(|i| i.key_pressed(Key::S));
        let resize = ctx
            .input(|i| i.modifiers.contains(Modifiers::ALT) && i.key_pressed(Key::M));
        if some_ctrl && up {
            self.overlays.open_switcher();
        } else if some_ctrl && pp {
            self.overlays.open_palette();
        } else if some_ctrl && ss {
            self.save_current();
        } else if resize {
            self.dark = !self.dark;
        }
    }

    fn overlay_pick(&mut self, ctx: &Context) {
        let pick = if self.overlays.switcher_open {
            self.overlays.draw_switcher(ctx, &self.vault)
        } else if self.overlays.palette_open {
            self.overlays.draw_palette(ctx)
        } else {
            OverlayPick::None
        };
        match pick {
            OverlayPick::Note(i) => self.open_note(i),
            OverlayPick::Command(c) => self.run_command(c),
            OverlayPick::None => {}
        }
    }
}

impl eframe::App for TintaApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        crate::theme::configure(ctx, self.dark);
        self.handle_shortcuts(ctx);

        SidePanel::left("explorer").min_width(220.0).show(ctx, |ui| {
            ui.heading("tinta");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut jump: Option<usize> = None;
                let mut last_folder = String::new();
                for (i, note) in self.vault.notes.iter().enumerate() {
                    if !note.folder.is_empty() && note.folder != last_folder {
                        ui.add_space(6.0);
                        ui.label(RichText::new(&note.folder).weak().strong().small());
                        last_folder = note.folder.clone();
                    }
                    let mut row =
                        egui::Button::selectable(self.sidebar_selected == i, &note.title)
                            .frame(false);
                    // Nota no guardada: punto rojo.
                    if self.current.as_ref().is_some_and(|c| c.path == note.path)
                        && self.text != self.saved_text
                    {
                        row = row.text_color(Color32::from_rgb(230, 120, 120));
                    }
                    if ui.add(row).clicked() {
                        jump = Some(i);
                    }
                }
                if let Some(i) = jump {
                    self.open_note(i);
                }
            });
        });

        self.overlay_pick(ctx);

        CentralPanel::default().show(ctx, |ui| {
            editor_main(self, ui);
        });

        if let Some(rel) = self.pending_jump.take() {
            self.open_note_rel(&rel);
        }
    }

}

fn editor_main(app: &mut TintaApp, ui: &mut Ui) {
    let Some(note) = app.current.clone() else {
        welcome(app, ui);
        return;
    };
    let dirty = app.text != app.saved_text;
    ui.horizontal(|ui| {
        ui.label(RichText::new(&note.rel_path).weak().monospace());
        if dirty {
            ui.label(
                RichText::new("• sin guardar")
                    .color(Color32::from_rgb(230, 120, 120)),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = if app.reading { "✎ Editar" } else { "👁 Leer" };
            if ui.button(label).clicked() {
                app.reading = !app.reading;
            }
        });
    });
    ui.separator();

    if app.reading {
        // Modo lectura: markdown renderizado, sin editar.
        egui::ScrollArea::vertical().show(ui, |ui| {
            render_markdown(ui, &app.text);
        });
    } else {
        // Modo edición: TextEdit multilinea mono, alto completo.
        let width = ui.available_width();
        let height = ui.available_height();
        let edit = egui::TextEdit::multiline(&mut app.text)
            .font(egui::TextStyle::Monospace)
            .hint_text("Escribe en Markdown…")
            .desired_width(width);
        ui.add_sized(egui::vec2(width, height), edit);
    }

    if app.show_backlinks {
        let title = note.title.clone();
        TopBottomPanel::bottom("backlinks")
            .show_inside(ui, |panel_ui| {
                panel_ui.label(RichText::new("Backlinks").weak().small());
                egui::ScrollArea::horizontal().show(panel_ui, |panel_ui| {
                    panel_ui.horizontal_wrapped(|row_ui| {
                        match app.backlinks.get(&title) {
                            None => {
                                row_ui.label(RichText::new("(sin backlinks)").weak());
                            }
                            Some(links) => {
                                for link in links {
                                    if row_ui.button(&link.from).clicked() {
                                        app.pending_jump = Some(link.from.clone());
                                    }
                                }
                            }
                        }
                    });
                });
            });
    } else {
        let _ = &app.backlinks;
    }
}

fn welcome(app: &mut TintaApp, ui: &mut Ui) {
    ui.add_space(40.0);
    ui.heading("tinta");
    ui.label(
        RichText::new("Vault de notas Markdown nativo en Rust — sin Electron")
            .weak(),
    );
    ui.add_space(16.0);
    if app.vault.notes.is_empty() {
        if ui.button("Abrir carpeta como vault…").clicked() {
            app.open_vault_dialog();
        }
        if ui.button("Reindexar").clicked() {
            app.reindex();
        }
    }
}
