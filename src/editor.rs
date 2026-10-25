//! Render de Markdown a widgets egui en modo lectura.
//! Wikilinks clicables, tags renderizados, bloques con resaltado syntect.

use egui::{Color32, FontId, RichText, Ui};
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use std::sync::OnceLock;

use crate::vault::Vault;

// syntect comparte gramáticas entre los frames.
fn syntax_set() -> &'static SyntaxSet {
    static SS: OnceLock<SyntaxSet> = OnceLock::new();
    SS.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn code_theme() -> &'static syntect::highlighting::Theme {
    static TS: OnceLock<ThemeSet> = OnceLock::new();
    let ts = TS.get_or_init(ThemeSet::load_defaults);
    ts.themes
        .get("base16-ocean.dark")
        .unwrap_or_else(|| ts.themes.values().next().unwrap())
}

#[derive(Clone, Copy, PartialEq)]
enum Block {
    Paragraph,
    Heading,
    Code,
}

/// Un segmento en línea: texto plano, wikilink o tag.
enum Seg {
    Plain(String),
    Wiki(String),
    Tag(String),
}

/// Divide una línea en segmentos interactuables.
fn split_inline(line: &str) -> Vec<Seg> {
    let mut segs = Vec::new();
    let mut plain = String::new();
    let mut rest = line;

    while !rest.is_empty() {
        if let Some(p) = rest.find("[[") {
            if let Some(end) = rest[p + 2..].find("]]") {
                if !plain.is_empty() {
                    segs.push(Seg::Plain(std::mem::take(&mut plain)));
                }
                let inner = &rest[p + 2..p + 2 + end];
                let target = inner.split('|').next().unwrap_or(inner).trim();
                if !target.is_empty() {
                    segs.push(Seg::Wiki(target.to_string()));
                }
                rest = &rest[p + 2 + end + 2..];
                continue;
            }
        }
        if let Some(p) = rest.find('#') {
            // '#' con letra después (no heading ya está filtrado por niveles arriba).
            let after = &rest[p + 1..];
            let tag: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || matches!(*c, '/' | '_' | '-'))
                .collect();
            if !tag.is_empty() && after.starts_with(|c: char| c.is_alphabetic()) {
                if !plain.is_empty() {
                    segs.push(Seg::Plain(std::mem::take(&mut plain)));
                }
                let tlen = tag.len();
                segs.push(Seg::Tag(tag));
                rest = &after[tlen..];
                continue;
            }
        }
        // Consumir al menos un char hacia plain.
        let take = rest.find(|c: char| c == '#' || c == '[').unwrap_or(rest.len());
        let take = take.max(1);
        plain.push_str(&rest[..take]);
        rest = &rest[take..];
    }
    if !plain.is_empty() {
        segs.push(Seg::Plain(plain));
    }
    segs
}

/// Renderiza markdown en modo lectura. Devuelve los wikilinks clicados.
pub fn render_markdown(ui: &mut Ui, source: &str, vault: &Vault) -> Vec<String> {
    let ss = syntax_set();
    let theme = code_theme();
    let mut clicked: Vec<String> = Vec::new();
    let mut state = Block::Paragraph;
    let mut paragraph = String::new();
    let mut heading_buf = String::new();
    let mut heading_level = 1u8;
    let mut code_buf = String::new();
    let mut code_lang = String::new();

    let flush = |paragraph: &mut String, clicked: &mut Vec<String>, ui: &mut Ui| {
        if paragraph.trim().is_empty() {
            paragraph.clear();
            return;
        }
        render_inline(ui, paragraph.trim(), vault, clicked);
        ui.add_space(2.0);
        paragraph.clear();
    };

    for event in Parser::new(source) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush(&mut paragraph, &mut clicked, ui);
                heading_level = level as u8;
                heading_buf.clear();
                state = Block::Heading;
            }
            Event::End(TagEnd::Heading(_)) => {
                if state == Block::Heading {
                    let text = std::mem::take(&mut heading_buf);
                    if !text.trim().is_empty() {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(text.trim())
                                .strong()
                                .size(heading_size(heading_level)),
                        );
                    }
                    ui.add_space(2.0);
                }
                state = Block::Paragraph;
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush(&mut paragraph, &mut clicked, ui);
                code_lang = code_lang_of(&kind);
                code_buf.clear();
                state = Block::Code;
            }
            Event::End(TagEnd::CodeBlock) => {
                if state == Block::Code {
                    let code = std::mem::take(&mut code_buf);
                    let code = code.trim_start_matches('\n');
                    ui.label(code_block_job(code, &code_lang, ss, theme));
                    ui.add_space(6.0);
                }
                state = Block::Paragraph;
            }
            Event::Text(t) => match state {
                Block::Heading => heading_buf.push_str(&t),
                Block::Code => code_buf.push_str(&t),
                Block::Paragraph => paragraph.push_str(&t),
            },
            Event::Code(c) => paragraph.push_str(&c),
            Event::SoftBreak | Event::HardBreak => paragraph.push('\n'),
            _ => {}
        }
    }
    flush(&mut paragraph, &mut clicked, ui);
    clicked
}

fn heading_size(level: u8) -> f32 {
    match level {
        1 => 26.0,
        2 => 22.0,
        3 => 19.0,
        _ => 17.0,
    }
}

fn code_lang_of(kind: &CodeBlockKind<'_>) -> String {
    match kind {
        CodeBlockKind::Fenced(info) => info
            .split([' ', ','])
            .next()
            .filter(|t| !t.is_empty())
            .unwrap_or("")
            .to_string(),
        CodeBlockKind::Indented => String::new(),
    }
}

/// Párrafo con wikilinks clicables y tags coloreados.
fn render_inline(ui: &mut Ui, text: &str, vault: &Vault, clicked: &mut Vec<String>) {
    ui.horizontal_wrapped(|row| {
        for seg in split_inline(text) {
            match seg {
                Seg::Plain(t) => {
                    row.label(t);
                }
                Seg::Wiki(target) => {
                    // Solo si es resoluble: wikilink clicable; else gris.
                    if vault.resolve_link(&target).is_some() {
                        if row
                            .button(
                                RichText::new(&target)
                                    .color(Color32::from_rgb(140, 170, 255)),
                            )
                            .clicked()
                        {
                            clicked.push(target);
                        }
                    } else {
                        row.label(
                            RichText::new(format!("[[{target}]]"))
                                .weak()
                                .small(),
                        );
                    }
                }
                Seg::Tag(tag) => {
                    row.label(
                        RichText::new(format!("#{tag}"))
                            .color(Color32::from_rgb(160, 140, 230)),
                    );
                }
            }
        }
    });
}

fn code_block_job(
    code: &str,
    lang: &str,
    ss: &SyntaxSet,
    theme: &syntect::highlighting::Theme,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let mono = FontId::monospace(13.0);
    let syntax = ss
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| ss.find_syntax_plain_text());
    let mut highlighter = HighlightLines::new(syntax, theme);
    for line in syntect::util::LinesWithEndings::from(code) {
        let Ok(ranges) = highlighter.highlight_line(line, ss) else {
            continue;
        };
        for (style, span) in ranges {
            let color =
                Color32::from_rgb(style.foreground.r, style.foreground.g, style.foreground.b);
            job.append(span, 0.0, egui::TextFormat::simple(mono.clone(), color));
        }
    }
    job
}
