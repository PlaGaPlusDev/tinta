//! Grafo de notas: nodos = notas, aristas = wikilinks resueltos.
//! Equivalente nativo (egui Painter) del grafo PixiJS de Obsidian.

use egui::{Color32, Pos2, Vec2};

use crate::index::outgoing_links;
use crate::vault::Vault;

/// Nodo del grafo con posición en un espacio normalizado [-1, 1]².
pub struct GraphNode {
    pub rel_path: String,
    pub title: String,
    pub pos: Vec2,
}

pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    /// Aristas por índice de nodo (from, to).
    pub edges: Vec<(usize, usize)>,
    /// aristas totales tras dedup.
    pub edge_count: usize,
}

impl Default for GraphData {
    fn default() -> Self {
        Self { nodes: Vec::new(), edges: Vec::new(), edge_count: 0 }
    }
}

/// Construye la estructura del grafo del vault: layout circular determinista.
pub fn build_graph(vault: &Vault) -> GraphData {
    let n = vault.notes.len();
    if n == 0 {
        return GraphData::default();
    }
    let nodes: Vec<GraphNode> = vault
        .notes
        .iter()
        .enumerate()
        .map(|(i, note)| {
            let angle = (i as f32) / (n as f32) * std::f32::consts::TAU;
            GraphNode {
                rel_path: note.rel_path.clone(),
                title: note.title.clone(),
                pos: Vec2::new(angle.cos(), angle.sin()),
            }
        })
        .collect();

    // Aristas: wikilinks resueltos entre notas, sin duplicados ni autoenlaces.
    let index_of = |rel: &str| nodes.iter().position(|nd| nd.rel_path == rel);
    let mut seen = std::collections::BTreeSet::new();
    let mut edges = Vec::new();
    for note in &vault.notes {
        let Ok(text) = std::fs::read_to_string(&note.path) else { continue };
        let from = index_of(&note.rel_path);
        let Some(from) = from else { continue };
        for link in outgoing_links(&text) {
            let Some(target) = vault.resolve_link(&link.target) else { continue };
            if target.rel_path == note.rel_path {
                continue;
            }
            let Some(to) = index_of(&target.rel_path) else { continue };
            let key = (from.min(to), from.max(to));
            if seen.insert(key) {
                edges.push(key);
            }
        }
    }
    let edge_count = edges.len();
    GraphData { nodes, edges, edge_count }
}

/// Dibuja el grafo en el área disponible. Devuelve el índice del nodo clicado.
pub fn draw_graph(ui: &mut egui::Ui, graph: &GraphData, current: Option<&str>) -> Option<usize> {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), ui.available_height()),
        egui::Sense::click(),
    );
    let visuals = ui.style().visuals.clone();

    if graph.nodes.is_empty() {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "vault vacío",
            egui::FontId::default(),
            visuals.weak_text_color(),
        );
        return None;
    }

    let center = rect.center();
    let radius = rect.size().min_component() * 0.5 - 28.0;
    let to_screen = |p: Vec2| -> Pos2 { Pos2::new(center.x + p.x * radius, center.y + p.y * radius) };

    // Aristas.
    let edge_color = Color32::from_rgba_unmultiplied(120, 120, 160, 90);
    for (a, b) in &graph.edges {
        let pa = to_screen(graph.nodes[*a].pos);
        let pb = to_screen(graph.nodes[*b].pos);
        ui.painter().line_segment([pa, pb], (1.0, edge_color));
    }

    // Nodos + etiquetas.
    let pointer = ui.input(|i| i.pointer.hover_pos());
    let mut clicked: Option<usize> = None;
    let highlight = Color32::from_rgb(230, 140, 140);
    let normal = Color32::from_rgb(140, 170, 255);
    for (i, node) in graph.nodes.iter().enumerate() {
        let p = to_screen(node.pos);
        let is_current = current.is_some_and(|c| c == node.rel_path);
        let hovered = pointer.is_some_and(|m| m.distance(p) < 14.0);
        let r = if is_current { 7.0 } else if hovered { 6.0 } else { 4.0 };
        let color = if is_current || hovered { highlight } else { normal };
        ui.painter().circle_filled(p, r, color);
        if hovered {
            ui.painter().text(
                p + Vec2::new(0.0, -r - 8.0),
                egui::Align2::CENTER_BOTTOM,
                &node.title,
                egui::FontId::proportional(12.0),
                visuals.strong_text_color(),
            );
        }
        if is_current || hovered {
            ui.painter().text(
                p + Vec2::new(0.0, r + 10.0),
                egui::Align2::CENTER_TOP,
                &node.title,
                egui::FontId::proportional(10.0),
                visuals.weak_text_color(),
            );
        }
    }

    // Click: nodo más cercano dentro del radio de interacción.
    if resp.clicked() {
        if let Some(m) = resp.interact_pointer_pos() {
            let mut best: Option<(f32, usize)> = None;
            for (i, node) in graph.nodes.iter().enumerate() {
                let d = m.distance(to_screen(node.pos));
                if d < 16.0 && best.map_or(true, |(bd, _)| d < bd) {
                    best = Some((d, i));
                }
            }
            if let Some((_, i)) = best {
                clicked = Some(i);
            }
        }
    }

    // Pie con número de nodos/aristas.
    ui.painter().text(
        rect.left_bottom() + Vec2::new(0.0, -2.0),
        egui::Align2::LEFT_BOTTOM,
        format!("{} notas · {} enlaces", graph.nodes.len(), graph.edge_count),
        egui::FontId::proportional(10.0),
        visuals.weak_text_color(),
    );
    clicked
}
