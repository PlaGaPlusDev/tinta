//! Aplicación principal: paneles, editor, backlinks, tags, outline, overlays.

use egui::{CentralPanel, Color32, Key, Modifiers, Panel, RichText, Ui};
use rfd::FileDialog;

use crate::commands::{today_iso, Command};
use crate::editor::render_markdown;
use crate::graph::{build_graph, draw_graph, GraphData};
use crate::index::{build_backlinks, build_tags, Backlinks, Tags};
use crate::search::{search_vault, outline, SearchHit};
use crate::switcher::{OverlayPick, Overlays};
use crate::vault::{Note, Vault};

pub struct TintaApp {
    vault: Vault,
    current: Option<Note>,
    text: String,
    saved_text: String,
    backlinks: Backlinks,
    tags: Tags,
    graph: GraphData,
    dark: bool,
    reading: bool,
    graph_view: bool,
    show_backlinks: bool,
    show_outline: bool,
    overlays: Overlays,
    search_hits: Vec<SearchHit>,
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
            tags: Tags::default(),
            graph: GraphData::default(),
            dark: true,
            reading: false,
            graph_view: false,
            show_backlinks: true,
            show_outline: true,
            overlays: Overlays::default(),
            search_hits: Vec::new(),
            sidebar_selected: 0,
            pending_jump: None,
        };
        if !app.vault.notes.is_empty() {
            app.open_note(0);
            app.reindex();
        }
        app
    }

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

    /// Navega por el objetivo de un wikilink (título o ruta).
    fn follow_link(&mut self, target: &str) {
        if let Some(note) = self.vault.resolve_link(target) {
            if let Ok(rel) = note.path.strip_prefix(&self.vault.root) {
                self.pending_jump = Some(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    fn save_current(&mut self) {
        if let Some(note) = &self.current {
            let _ = std::fs::write(&note.path, &self.text);
            self.saved_text = self.text.clone();
            self.reindex();
        }
    }

    fn reindex(&mut self) {
        self.vault.scan();
        self.backlinks = build_backlinks(&self.vault);
        self.tags = build_tags(&self.vault);
        self.graph = build_graph(&self.vault);
    }

    fn open_vault_dialog(&mut self) {
        if let Some(folder) = FileDialog::new().pick_folder() {
            self.vault = Vault::open(&folder.to_string_lossy());
            self.current = None;
            self.text.clear();
            self.saved_text.clear();
            if !self.vault.notes.is_empty() {
                self.open_note(0);
                self.reindex();
            }
        }
    }

    fn new_note(&mut self) {
        self.create_note_at("nueva-nota.md", "nueva-nota");
    }

    /// Nota diaria: "YYYY-MM-DD.md" en la raíz del vault.
    fn daily_note(&mut self) {
        self.create_note_at(&format!("{}.md", today_iso()), "el día de hoy");
    }

    fn create_note_at(&mut self, filename: &str, label: &str) {
        let mut path = self.vault.root.join(filename);
        let stem = filename.trim_end_matches(".md");
        let mut n = 1;
        while path.exists() {
            path = self.vault.root.join(format!("{stem}-{n}.md"));
            n += 1;
        }
        let _ = std::fs::write(&path, "");
        self.reindex();
        if let Ok(rel) = path.strip_prefix(&self.vault.root) {
            self.open_note_rel(&rel.to_string_lossy().replace('\\', "/"));
        } else {
            eprintln!("no se pudo abrir {label}");
        }
    }

    fn run_command(&mut self, command: Command) {
        match command {
            Command::TogglePreview => self.reading = !self.reading,
            Command::ToggleBacklinks => self.show_backlinks = !self.show_backlinks,
            Command::ToggleOutline => self.show_outline = !self.show_outline,
            Command::ToggleGraph => self.graph_view = !self.graph_view,
            Command::VaultSearch => self.overlays.open_search(None),
            Command::RescanVault => self.reindex(),
            Command::OpenVault => self.open_vault_dialog(),
            Command::NewNote => self.new_note(),
            Command::DailyNote => self.daily_note(),
            Command::ToggleTheme => self.dark = !self.dark,
            Command::Quit => std::process::exit(0),
        }
    }

    fn handle_shortcuts(&mut self, ui: &mut Ui) {
        if self.overlays.any_open() {
            return;
        }
        let ctx = ui.ctx();
        let (ctrl, shift) = ctx.input(|i| {
            (
                i.modifiers.contains(Modifiers::CTRL),
                i.modifiers.contains(Modifiers::SHIFT),
            )
        });
        let pressed = ctx.input(|i| {
            (
                i.key_pressed(Key::O),
                i.key_pressed(Key::P),
                i.key_pressed(Key::S),
                i.key_pressed(Key::F),
            )
        });
        let (o, p, s, f) = pressed;
        if ctrl && f {
            self.overlays.open_search(None);
        } else if ctrl && o {
            self.overlays.open_switcher();
        } else if ctrl && p {
            self.overlays.open_palette();
        } else if ctrl && s {
            self.save_current();
        } else if ctrl && shift {
            // Reservado.
        }
    }

    fn overlay_picker(&mut self, ui: &mut Ui) {
        let pick = if self.overlays.switcher_open {
            self.overlays.draw_switcher(ui.ctx(), &self.vault)
        } else if self.overlays.palette_open {
            self.overlays.draw_palette(ui.ctx())
        } else if self.overlays.search_open {
            // La búsqueda recalcula por frame con el query tecleado.
            self.search_hits = search_vault(&self.vault, &self.overlays.query, 80);
            self.overlays.draw_search(ui.ctx(), &self.search_hits)
        } else {
            OverlayPick::None
        };
        match pick {
            OverlayPick::Note(i) => self.open_note(i),
            OverlayPick::Command(c) => self.run_command(c),
            OverlayPick::Search(i) => {
                if let Some(hit) = self.search_hits.get(i) {
                    let rel = hit.rel_path.clone();
                    self.open_note_rel(&rel);
                }
            }
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
        if self.show_outline && self.current.is_some() {
            Panel::right("outline")
                .min_size(180.0)
                .resizable(true)
                .show(ui, |ui| self.outline_panel(ui));
        }

        self.overlay_picker(ui);
        self.handle_shortcuts(ui);

        CentralPanel::default_margins().show(ui, |ui| editor_main(self, ui));

        if let Some(rel) = self.pending_jump.take() {
            self.open_note_rel(&rel);
        }
    }
}

impl TintaApp {
    /// Explorador lateral agrupado por carpetas.
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

    /// Panel derecho: esquema (headings) de la nota abierta.
    fn outline_panel(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("Esquema").weak().small());
        let headings = outline(&self.text);
        let resp = ui.add(
            egui::Button::selectable(self.reading, "Modo lectura")
                .frame(false),
        );
        if resp.clicked() {
            self.reading = !self.reading;
        }
        ui.separator();
        let available = ui.available_width();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for heading in &headings {
                let indent = ((heading.level.min(4) - 1) as f32) * 10.0;
                let label = egui::RichText::new(&heading.text).size(match heading.level {
                    1 => 15.0,
                    2 => 14.0,
                    _ => 13.0,
                });
                ui.indent(format!("outline-{}", heading.line_no), |row| {
                    row.set_min_width(available - indent);
                    if row.add(egui::Button::selectable(false, label).frame(false)).clicked() {
                        // En modo lectura abrimos; salto de caret en V2.
                        if self.reading {
                            // scroll por anchor aún no implementado.
                        }
                    }
                });
            }
            if headings.is_empty() {
                ui.label(RichText::new("(sin encabezados)").weak());
            }
        });
    }
}

fn editor_main(app: &mut TintaApp, ui: &mut Ui) {
    if app.graph_view {
        ui.horizontal(|ui| {
            if ui.button("← Editor").clicked() {
                app.graph_view = false;
            }
            ui.label(RichText::new("Grafo de notas — click en un nodo para abrirlo").weak().small());
        });
        let current = app.current.as_ref().map(|n| n.rel_path.clone());
        let clicked = draw_graph(ui, &app.graph, current.as_deref());
        if let Some(i) = clicked {
            let rel = app.graph.nodes[i].rel_path.clone();
            app.open_note_rel(&rel);
        }
        return;
    }
    let Some(note) = app.current.clone() else {
        welcome(app, ui);
        return;
    };
    let dirty = app.text != app.saved_text;

    ui.horizontal(|ui| {
        ui.label(RichText::new(&note.rel_path).weak().monospace());
        if dirty {
            ui.label(RichText::new("• sin guardar").color(Color32::from_rgb(230, 120, 120)));
        }
        let words = app.text.split_whitespace().count();
        ui.label(RichText::new(format!("{words} palabras")).weak().small());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✦ Grafo").clicked() {
                app.graph_view = !app.graph_view;
            }
            let label = if app.reading { "✎ Editar" } else { "👁 Leer" };
            if ui.button(label).clicked() {
                app.reading = !app.reading;
            }
        });
    });
    ui.separator();

    if app.reading {
        let clicked = egui::ScrollArea::vertical()
            .show(ui, |ui| render_markdown(ui, &app.text, &app.vault))
            .inner;
        for target in clicked.into_iter() {
            app.follow_link(&target);
        }
    } else {
        let width = ui.available_width();
        let height = ui.available_height();
        let edit = egui::TextEdit::multiline(&mut app.text)
            .font(egui::TextStyle::Monospace)
            .hint_text("Escribe en Markdown…")
            .desired_width(width);
        ui.add_sized(egui::vec2(width, height), edit);
    }

    // Backlinks + tags de la nota actual.
    if app.show_backlinks {
        let title = note.title.clone();
        ui.separator();
        ui.horizontal_wrapped(|row| {
            row.label(RichText::new("Backlinks:").weak().small());
            match app.backlinks.get(&title) {
                None => {
                    row.label(RichText::new("(sin backlinks)").weak());
                }
                Some(links) => {
                    for link in links {
                        if row.button(&link.from).clicked() {
                            app.pending_jump = Some(link.from.clone());
                        }
                    }
                }
            }
        });
        ui.horizontal_wrapped(|row| {
            row.label(RichText::new("Tags:").weak().small());
            let note_tags: Vec<String> = crate::index::tags_in_text(&app.text);
            if note_tags.is_empty() {
                row.label(RichText::new("(sin tags)").weak());
            } else {
                for tag in note_tags {
                    if row
                        .button(RichText::new(format!("#{tag}")).weak())
                        .clicked()
                    {
                        app.overlays.open_search(Some(tag));
                    }
                }
            }
        });
    }
}

fn welcome(app: &mut TintaApp, ui: &mut Ui) {
    ui.add_space(40.0);
    ui.heading("tinta");
    ui.label(RichText::new("Vault de notas Markdown nativo en Rust — sin Electron").weak());
    ui.add_space(16.0);
    if ui.button("Abrir carpeta como vault…").clicked() {
        app.open_vault_dialog();
    }
    if ui.button("Reindexar").clicked() {
        app.reindex();
    }
    ui.add_space(8.0);
    if ui.button("Nota del día").clicked() {
        app.daily_note();
    }
}
