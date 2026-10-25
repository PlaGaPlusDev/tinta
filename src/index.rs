//! Índice de wikilinks y backlinks por título de nota.

use std::collections::BTreeMap;

use crate::vault::Vault;

/// Un vínculo saliente `[texto](target)` o `[[target]]`.
#[derive(Debug, Clone)]
pub struct Outgoing {
    /// Título/literal objetivo tal como aparece en el texto.
    pub target: String,
    /// Contexto de la línea.
    pub line: String,
}

/// Referencia entrante hacia una nota.
#[derive(Debug, Clone)]
pub struct Backlink {
    /// rel_path de la nota origen.
    pub from: String,
    /// Línea de contexto del enlace.
    pub context: String,
}

/// Mapa título → backlinks ordenados por nota origen.
pub type Backlinks = BTreeMap<String, Vec<Backlink>>;

/// Extrae los enlaces salientes de un texto de nota.
pub fn outgoing_links(text: &str) -> Vec<Outgoing> {
    let mut out = Vec::new();
    for line in text.lines() {
        // Wikilinks en líneas: [[Nombre]] y [[Nombre|alias]].
        let mut rest = line;
        while let Some(start) = rest.find("[[") {
            if let Some(end) = rest[start + 2..].find("]]") {
                let inner = &rest[start + 2..start + 2 + end];
                let target = inner.split('|').next().unwrap_or(inner).trim().to_string();
                if !target.is_empty() {
                    out.push(Outgoing { target, line: line.trim().to_string() });
                }
                rest = &rest[start + 2 + end + 2..];
            } else {
                break;
            }
        }
        // Enlaces markdown locales: [texto](ruta.md)
        if let Some(markdown) = pulldown_cmark::Parser::new(line)
            .filter_map(|e| match e {
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { dest_url, .. }) => {
                    Some(dest_url.into_string())
                }
                _ => None,
            })
            .find(|u| u.ends_with(".md"))
        {
            let target = markdown.trim().trim_start_matches("./").to_string();
            if !target.is_empty() {
                out.push(Outgoing { target, line: line.trim().to_string() });
            }
        }
    }
    out
}

/// Construye el índice de backlinks de todo el vault.
pub fn build_backlinks(vault: &Vault) -> Backlinks {
    let mut index: Backlinks = BTreeMap::new();
    for note in &vault.notes {
        let Ok(text) = std::fs::read_to_string(&note.path) else {
            continue;
        };
        for link in outgoing_links(&text) {
            if let Some(target) = vault.resolve_link(&link.target) {
                index
                    .entry(target.title.clone())
                    .or_default()
                    .push(Backlink { from: note.rel_path.clone(), context: link.line.clone() });
            }
        }
    }
    index
}
