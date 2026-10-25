//! Watcher de cambios externos en el vault (equivalente a los eventos
//! `onCreate`/`onModify`/`onDelete` del Vault de Obsidian).
//! Thread en background con inotify (crate notify) + canal a la app.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

use notify::{Event, EventKind, RecursiveMode, RecommendedWatcher, Watcher};

pub struct VaultWatcher {
    rx: Receiver<PathBuf>,
    /// Se conserva activo mientras exista el struct.
    _watcher: RecommendedWatcher,
}

impl VaultWatcher {
    /// Empieza a vigilar `root` de forma recursiva. Devuelve None si el
    /// backend del sistema no está disponible.
    pub fn start(root: &Path) -> Option<Self> {
        let (tx, rx) = channel();
        let mut watcher =
            notify::recommended_watcher(move |res: Result<Event, _>| {
                if let Ok(ev) = res {
                    // Los eventos de acceso (lecturas) no cuentan.
                    if matches!(ev.kind, EventKind::Access(_)) {
                        return;
                    }
                    for p in ev.paths {
                        if p.extension().is_some_and(|e| e == "md") {
                            let _ = tx.send(p);
                        }
                    }
                }
            })
            .ok()?;
        watcher.watch(root, RecursiveMode::Recursive).ok()?;
        Some(Self { rx, _watcher: watcher })
    }

    /// Drena los eventos pendientes (solo .md) sin bloquear el frame.
    /// Tope de 64 por ciclo para acotar RAM ante ráfagas.
    pub fn poll(&mut self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        while out.len() < 64 {
            match self.rx.try_recv() {
                Ok(p) => out.push(p),
                Err(_) => break,
            }
        }
        out
    }
}
