//! Catálogo de comandos de la paleta (Ctrl+P).

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    TogglePreview,
    ToggleBacklinks,
    RescanVault,
    OpenVault,
    NewNote,
    ToggleTheme,
    Quit,
}

impl Command {
    pub fn label(&self) -> &'static str {
        match self {
            Command::TogglePreview => "Vista: alternar edición/lectura",
            Command::ToggleBacklinks => "Backlinks: mostrar/ocultar panel",
            Command::RescanVault => "Vault: reindexar notas",
            Command::OpenVault => "Vault: abrir otra carpeta…",
            Command::NewNote => "Nota: crear nueva en la raíz",
            Command::ToggleTheme => "Tema: alternar claro/oscuro",
            Command::Quit => "Tinta: salir",
        }
    }
}

/// Base de topónimos (id, etiqueta) para búsqueda fuzzy de la paleta.
pub fn all_commands() -> Vec<(Command, String)> {
    [
        Command::TogglePreview,
        Command::ToggleBacklinks,
        Command::RescanVault,
        Command::OpenVault,
        Command::NewNote,
        Command::ToggleTheme,
        Command::Quit,
    ]
    .into_iter()
    .map(|c| (c, c.label().to_string()))
    .collect()
}
