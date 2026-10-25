# Documentación técnica de tinta

## Decisiones de diseño

1. **egui + eframe (glow)** — immediate mode, GPU backend GL. Sin wgpu para
   reducir RAM/compile time. Cero runtime web.
2. **Suite de motor markdown** — pulldown-cmark (CommonMark) para lectura;
   edición es texto plano con fuente mono (V2: CodeMirror-like features).
3. **Índice backlinks** — reconstruido tras cada guardado. BTreeMap título→
   Vec<Backlink>. Bounded: recomputar O(n_notes) lecturas.
4. **Wikilinks** — resolución primera por título exacto, luego por ruta
   relativa con `starts_with` (compatibilidad con nombres ambiguos).
5. **Fuzzy** — subsecuencia con bonus por contigüidad y posición temprana;
   sin dependencias.
6. **RAM** — escaneo streaming (walkdir), syntect con OnceLock singleton,
   sin arranques por frame. El cálculo pesado (build release) se hace en
   GitHub Actions (2 CI min/build aprox).

## Trade-offs frente a Obsidian
| Aspecto | Obsidian | tinta |
|---|---|---|
| RAM | 150–400 MB | 40–90 MB |
| Runtime | Electron+Node | nativo |
| Editor | CodeMirror | TextEdit (V2 mejora) |
| Plugins | JS sandboxing | V2: proces dinámico en Rust |

## Síntesis del análisis REA original (Obsidian 1.13.7)
Ver `.project.md` y DevLog. Evidence id: `ev_8a6ea231f23118a659e1c6babd1f2295153dc306055e57e82c12220941cf2586`.
