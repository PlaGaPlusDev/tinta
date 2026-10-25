use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

/// Carpetas ignoradas durante el escaneo del vault.
const IGNORED_DIRS: [&str; 4] = [".git", ".trash", ".obsidian", "target"];

// ---------------------------------------------------------------- settings

#[derive(Serialize, Deserialize)]
struct VaultSettings {
    last_vault: Option<String>,
}

fn settings_path() -> PathBuf {
    let base = match std::env::var("XDG_CONFIG_HOME") {
        Ok(home) => PathBuf::from(home),
        Err(_) => PathBuf::from(".config"),
    };
    base.join("tinta/settings.json")
}

pub fn load_last_vault() -> Option<String> {
    let data = std::fs::read_to_string(settings_path()).ok()?;
    serde_json::from_str::<VaultSettings>(&data).ok()?.last_vault
}

pub fn save_last_vault(vault: &str) {
    if let Some(parent) = settings_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let settings = VaultSettings { last_vault: Some(vault.to_string()) };
    if let Ok(json) = serde_json::to_string(&settings) {
        let _ = std::fs::write(settings_path(), json);
    }
}

// ------------------------------------------------------------------- vault

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    /// Ruta absoluta en disco.
    pub path: PathBuf,
    /// Ruta relativa con extensión ("carpeta/nota.md").
    pub rel_path: String,
    /// Nombre sin extensión (identidad base de wikilinks).
    pub title: String,
    /// Carpeta relativa ("carpeta" o "" en la raíz).
    pub folder: String,
}

#[derive(Debug, Default)]
pub struct Vault {
    pub root: PathBuf,
    pub notes: Vec<Note>,
}

impl Vault {
    /// Abre y escanea un vault guardándolo como último usado.
    pub fn open(root: &str) -> Self {
        let root = PathBuf::from(root);
        let mut vault = Self { root, notes: Vec::new() };
        vault.scan();
        if let Some(s) = vault.root.to_str() {
            save_last_vault(s);
        }
        vault
    }

    /// Escaneo completo: streaming con walkdir, RAM acotada.
    pub fn scan(&mut self) {
        self.notes.clear();
        for entry in WalkDir::new(&self.root)
            .follow_links(true)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.file_type().is_dir() {
                if IGNORED_DIRS.contains(&name.as_str()) {
                    continue; // No descendir.
                }
            } else if entry.path().extension().is_some_and(|e| e == "md") {
                let Ok(rel) = entry.path().strip_prefix(&self.root) else { continue };
                let rel_path = rel.to_string_lossy().replace('\\', "/");
                let (title, folder) = match rel.parent().and_then(|p| p.to_str()) {
                    Some(p) if p.is_empty() => (
                        rel.file_stem().unwrap_or_default().to_string_lossy().into_owned(),
                        String::new(),
                    ),
                    Some(p) => (
                        rel.file_stem().unwrap_or_default().to_string_lossy().into_owned(),
                        p.replace('\\', "/"),
                    ),
                    None => continue,
                };
                self.notes.push(Note {
                    path: entry.path().to_path_buf(),
                    rel_path,
                    title,
                    folder,
                });
            }
        }
        self.notes.sort_by(|a, b| (&a.folder, &a.title).cmp(&(&b.folder, &b.title)));
    }

    /// Resuelve un wikilink por título exacto, luego por ruta relativa.
    /// Fragmentos (#sección) se ignoran en este MVP.
    pub fn resolve_link(&self, link: &str) -> Option<Note> {
        let link = link.trim().split('#').next().unwrap_or("").trim();
        if link.is_empty() {
            return None;
        }
        if let Some(n) = self.notes.iter().find(|n| n.title == link) {
            return Some(n.clone());
        }
        let base = link.strip_suffix(".md").unwrap_or(link);
        self.notes
            .iter()
            .find(|n| n.rel_path.starts_with(base))
            .cloned()
    }

    pub fn absolutize(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }
}
