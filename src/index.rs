//! Índice de wikilinks, backlinks y tags del vault.

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

/// Mapa tag (sin #) → rutas relativas de notas que lo usan.
pub type Tags = BTreeMap<String, Vec<String>>;

/// Extrae los enlaces salientes de un texto de nota.
pub fn outgoing_links(text: &str) -> Vec<Outgoing> {
    let mut out = Vec::new();
    for line in text.lines() {
        // Wikilinks [[Nombre]] y [[Nombre|alias]].
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
        for event in pulldown_cmark::Parser::new(line) {
            if let pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { dest_url, .. }) =
                event
            {
                if dest_url.ends_with(".md") && !dest_url.contains("://") {
                    let target = dest_url.trim_start_matches("./").to_string();
                    if !target.is_empty() {
                        out.push(Outgoing { target, line: line.trim().to_string() });
                    }
                }
            }
        }
    }
    out
}

/// Extrae tags **#foo** de un texto: excluir heading lines ``# Título``.
pub fn tags_in_text(text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for line in text.lines() {
        // Las líneas de encabezado (# Título) no cuentan como tags.
        if line.trim_start().starts_with('#') && !line.contains(" #") {
            continue;
        }
        let mut rest = line;
        while let Some(start) = rest.find('#') {
            let after = &rest[start + 1..];
            let tag: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || '/' == *c || '_' == *c || '-' == *c)
                .collect();
            if tag.is_empty() || tag.chars().next().map_or(true, |c| c.is_ascii_digit()) {
                // '#' seguido de nada o número: avanza y sigue.
                rest = after;
                continue;
            }
            tags.push(tag);
            rest = after;
        }
    }
    tags.sort();
    tags.dedup();
    tags
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

/// Construye el índice de tags de todo el vault.
pub fn build_tags(vault: &Vault) -> Tags {
    let mut tags: Tags = BTreeMap::new();
    for note in &vault.notes {
        let Ok(text) = std::fs::read_to_string(&note.path) else {
            continue;
        };
        for tag in tags_in_text(&text) {
            tags.entry(tag).or_default().push(note.rel_path.clone());
        }
    }
    tags
}
