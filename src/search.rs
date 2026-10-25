//! Búsqueda textual en el vault. Streaming y RAM acotada,
//! atravesando un archivo por vez a la vez.

use std::path::Path;

use crate::vault::Vault;

/// Un resultado de búsqueda.
#[derive(Debug, Clone)]
pub struct SearchHit {
    /// Ruta relativa de la nota.
    pub rel_path: String,
    /// Línea numérica (1-based).
    pub line_no: usize,
    /// Texto de la línea (trimmed).
    pub line: String,
}

/// Busca `query` (case-insensitive) como substring en los cuerpos de las notas.
/// límite por archivos de tamaño proporcional evitando archivos grandes.
pub fn search_vault(vault: &Vault, query: &str, max_hits: usize) -> Vec<SearchHit> {
    let q = query.to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for note in &vault.notes {
        if hits.len() >= max_hits {
            break;
        }
        if file_too_big(&note.path) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&note.path) else {
            continue;
        };
        for (line_no, line) in text.lines().enumerate() {
            if line.to_lowercase().contains(&q) {
                hits.push(SearchHit {
                    rel_path: note.rel_path.clone(),
                    line_no: line_no + 1,
                    line: line.trim().to_string(),
                });
                if hits.len() >= max_hits {
                    break;
                }
            }
        }
    }
    hits
}

const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
fn file_too_big(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.len() > MAX_FILE_BYTES)
        .unwrap_or(false)
}

/// Esquema de encabezados: índice de un texto (outline de nota).
#[derive(Debug, Clone)]
pub struct Heading {
    /// Nivel 1–6.
    pub level: u8,
    pub text: String,
    /// Línea 1-based.
    pub line_no: usize,
}

/// Extrae el esquema (Encabezados) de una nota que se está editando.
pub fn outline(text: &str) -> Vec<Heading> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some(heading) = parse_heading(trimmed) {
            out.push(Heading { level: heading.0, text: heading.1.to_string(), line_no: line_no + 1 });
        }
    }
    out
}

fn parse_heading(trimmed: &str) -> Option<(u8, &str)> {
    let level = trimmed.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&level) {
        let rest = &trimmed[level..];
        if rest.starts_with(' ') {
            return Some((level as u8, rest.trim()));
        }
    }
    None
}
