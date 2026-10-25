//! Catálogo de comandos de la paleta (Ctrl+P) + fecha util.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    TogglePreview,
    ToggleBacklinks,
    ToggleOutline,
    VaultSearch,
    RescanVault,
    OpenVault,
    NewNote,
    DailyNote,
    ToggleTheme,
    Quit,
}

impl Command {
    pub fn label(&self) -> &'static str {
        match self {
            Command::TogglePreview => "Vista: alternar edición/lectura",
            Command::ToggleBacklinks => "Backlinks: mostrar/ocultar panel",
            Command::ToggleOutline => "Esquema: mostrar/ocultar panel",
            Command::VaultSearch => "Vault: buscar en todas las notas (Ctrl+Shift+F)",
            Command::RescanVault => "Vault: reindexar notas",
            Command::OpenVault => "Vault: abrir otra carpeta…",
            Command::NewNote => "Nota: crear nueva en la raíz",
            Command::DailyNote => "Nota: abrir nota del día (diaria)",
            Command::ToggleTheme => "Tema: alternar claro/oscuro",
            Command::Quit => "Tinta: salir",
        }
    }

    pub fn all() -> Vec<(Command, String)> {
        [
            Command::TogglePreview,
            Command::ToggleBacklinks,
            Command::ToggleOutline,
            Command::VaultSearch,
            Command::RescanVault,
            Command::OpenVault,
            Command::NewNote,
            Command::DailyNote,
            Command::ToggleTheme,
            Command::Quit,
        ]
        .into_iter()
        .map(|c| (c, c.label().to_string()))
        .collect()
    }
}

/// Fecha UTC de hoy como "2026-10-25" sin dependencias externas.
/// Convierte días desde epoch a civil (algoritmo de Howard Hinnant).
pub fn today_iso() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let days = (now.as_secs() / 86_400) as i64;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::civil_from_days;

    #[test]
    fn epoch_break() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19), (1970, 1, 20));
        assert_eq!(civil_from_days(365), (1971, 1, 1));
    }
}
