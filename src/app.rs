//! Aplicación principal: paneles, editor, backlinks y overlays.
//! API egui 0.37: App::ui, Panel unificado, CentralPanel::default_margins.

use egui::{CentralPanel, Color32, Key, Modifiers, Panel, RichText, Ui};
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
        let vault = match vault_path {
            Some(p) => Vault::open(&p),
            None => Vault::default(),
        };
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
        if !app.vault.notes.is_empty() {
            app.open_note(0);
            app.backlinks = build_backlinks(&app.vault);
        }
        app
    }

    /// Abre la nota por índice del vault escaneado.
    fn open_note(&mut self, index: usize) {
        let Some(note) = self.vault.notes.get(index).cloned() else {
            return;
        };
        self.text = std::fs::read_to_string(&note.path).unwrap_or_default();
        self.saved_text = self.text.clone();
        self.current = Some(note);
        self.sidebar_selected = index;
    }

    fn open_note_rel(&mut self, rel_path: &str) {
        if let Some(i) = self
            .vault
            .notes
            .iter()
            .position(|n| n.rel_path == rel_path)
        {
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
            self.current = None;
            self.text.clear();
            self.saved_text.clear();
            if !self.vault.notes.is_empty() {
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
        self.reindex();
        if let Ok(rel) = path.strip_prefix(&self.vault.root) {
            self.open_note_rel(&rel.to_string_lossy().replace('\\', "/"));
        }
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

    fn handle_shortcuts(&mut self, ui: &mut Ui) {
        if self.overlays.any_open() {
            return; // Esc/Enter/↑/↓ los gestiona el overlay.
        }
        let Some(ctx_ref) = Some(ui.ctx()) else {
            return;
        };
        let ctrl = ctx_ref.input(|i| i.modifiers.contains(Modifiers::CTRL));
        let go = ctx_ref.input(|i| i.key_pressed(Key::O));
        let pp = ctx_ref.input(|i| i.key_pressed(Key::P));
        let ss = ctx_ref.input(|i| i.key_pressed(Key::S));
        if ctrl && go {
            self.overlays.open_switcher();
        } else if ctrl && pp {
            self.overlays.open_palette();
        } else if ctrl && ss {
            self.save_current();
        }
    }

    fn overlay_picker(&mut self, ui: &mut Ui) {
        let pick = if self.overlays.switcher_open {
            self.overlays.draw_switcher(ui.ctx(), &self.vault)
        } else if self.overlays.palette_open {
            self.overlays.draw_palette(ui.ctx())
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
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        crate::theme::configure(ui.ctx(), self.dark);

        Panel::left("explorer")
            .min_size(220.0)
            .resizable(true)
            .show(ui, |ui| self.sidebar(ui));

        self.overlay_picker(ui);
        self.handle_shortcuts(ui);

        CentralPanel::default_margins().show(ui, |ui| editor_main(self, ui));

        if let Some(rel) = self.pending_jump.take() {
            self.open_note_rel(&rel);
        }
    }
}

impl TintaApp {
    /// Explorador lateral con notas agrupadas por carpeta.
    fn sidebar(&mut self, ui: &mut Ui) {
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
                let dirty = self.current.as_ref().is_some_and(|c| c.path == note.path)
                    && self.text != self.saved_text;
                let title = if dirty {
                    format!("{} •", note.title)
                } else {
                    note.title.clone()
                };
                if ui
                    .add(
                        egui::Button::selectable(self.sidebar_selected == i, title)
                            .frame(false),
                    )
                    .clicked()
                {
                    jump = Some(i);
                }
            }
            if let Some(i) = jump {
                self.open_note(i);
            }
        });
    }
}

fn editor_main(app: &mut TintaApp, ui: &mut Ui) {
    let Some(note) = app.current.clone() else {
        welcome(app, ui);
        return;
    };
    let dirty = app.text != app.saved_text;

    // Cabecera.
    ui.horizontal(|ui| {
        ui.label(RichText::new(&note.rel_path).weak().monospace());
        if dirty {
            ui.label(
                RichText::new("• sin guardar").color(Color32::from_rgb(230, 120, 120)),
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
        egui::ScrollArea::vertical().show(ui, |ui| {
            render_markdown(ui, &app.text);
        });
    } else {
        let width = ui.available_width();
        let height = ui.available_height();
        let edit = egui::TextEdit::multiline(&mut app.text)
            .font(egui::TextStyle::Monospace)
            .hint_text("Escribe en Markdown…")
            .desired_width(width);
        ui.add_sized(egui::vec2(width, height), edit);
    }

    // Backlinks como sección plana.
    if app.show_backlinks {
        let title = note.title.clone();
        ui.separator();
        ui.horizontal_wrapped(|row_ui| {
            row_ui.label(RichText::new("Backlinks:").weak().small());
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
    }
}

fn welcome(app: &mut TintaApp, ui: &mut Ui) {
    ui.add_space(40.0);
    ui.heading("tinta");
    ui.label(
        RichText::new("Vault de notas Markdown nativo en Rust — sin Electron").weak(),
    );
    ui.add_space(16.0);
    if ui.button("Abrir carpeta como vault…").clicked() {
        app.open_vault_dialog();
    }
    if ui.button("Reindexar").clicked() {
        app.reindex();
    }
}
