# tinta

> Vault de notas Markdown nativo en Rust (egui). Libre de Electron.

## Tech Stack
- **Language:** Rust 1.97+
- **UI:** egui 0.37 / eframe (backend glow, sin wgpu)
- **Markdown:** pulldown-cmark
- **Highlight:** syntect
- **FS:** walkdir

## Getting Started

### Prerequisites
- Rust stable (rustup)
- Linux X11 o Wayland

### Run
```bash
cargo run --release            # abre el último vault usado
cargo run --release -- ~/ruta/vault
```

### Test
```bash
cargo test
```

## Project Structure
```
tinta/
├── src/
│   ├── main.rs      → args + arranque eframe
│   ├── app.rs       → paneles, editor, backlinks, shortcuts
│   ├── vault.rs     → escaneo de vault, settings
│   ├── index.rs     → wikilinks y backlinks
│   ├── fuzzy.rs     → búsqueda subsecuencia para switcher
│   ├── editor.rs    → render markdown (lectura) + syntect
│   ├── switcher.rs  → overlays switcher/paleta
│   ├── commands.rs  → comandos de paleta
│   └── theme.rs     → visuals claro/oscuro
├── docs/index.md
├── DevLog/
└── .github/workflows/ci.yml
```

## Atajos
| Atajo | Acción |
|-------|--------|
| Ctrl+O | Quick switcher |
| Ctrl+P | Paleta de comandos |
| Ctrl+S | Guardar nota |
| Esc | Cerrar overlay |

## License
MIT
