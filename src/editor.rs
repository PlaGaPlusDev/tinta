//! Render de Markdown a widgets egui en modo lectura.
//! Bloques de código con resaltado syntect → LayoutJob.

use egui::{Color32, FontId, RichText, Ui};
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use syntect::highlighting::{HighlightLines, SyntaxSet, ThemeSet};
use std::sync::OnceLock;

// syntect carga sus gramáticas una sola vez e inmediato-mode reutiliza.
fn syntax_set() -> &'static SyntaxSet {
    static SS: OnceLock<SyntaxSet> = OnceLock::new();
    SS.get_or_init(SyntaxSet::load_defaults_newlines).clone()
}

fn code_theme() -> &'static syntect::highlighting::Theme {
    static TS: OnceLock<ThemeSet> = OnceLock::new();
    let ts = TS.get_or_init(ThemeSet::load_defaults);
    ts.themes
        .get("base16-ocean.dark")
        .unwrap_or_else(|| ts.themes.values().next().unwrap())
}

/// Estado del render por bloques durante el pase del parser.
enum Block {
    Paragraph,
    Heading { level: u8, buf: String },
    Code { lang: String, buf: String },
}

/// Renderiza un texto Markdown en modo lectura.
pub fn render_markdown(ui: &mut Ui, source: &str) {
    let ss = syntax_set().clone();
    let theme = code_theme();
    let mut state = Block::Paragraph;
    let mut paragraph = String::new();

    for event in Parser::new(source) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_paragraph(ui, &mut paragraph);
                state = Block::Heading { level: level as u8, buf: String::new() };
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Block::Heading { level, buf } = state {
                    let text = std::mem::take(&mut buf);
                    if !text.trim().is_empty() {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(text.trim())
                                .strong()
                                .size(heading_size(level)),
                        );
                    }
                    ui.add_space(2.0);
                }
                state = Block::Paragraph;
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_paragraph(ui, &mut paragraph);
                state = Block::Code { lang: code_lang(&kind), buf: String::new() };
            }
            Event::End(TagEnd::CodeBlock) => {
                let Block::Code { lang, buf } = state else {
                    return;
                };
                render_code_block(ui, &buf, &lang, &ss, theme);
                state = Block::Paragraph;
            }
            Event::Text(t) => match state {
                Block::Heading { buf, .. } => buf.push_str(&t),
                Block::Code { buf, .. } => buf.push_str(&t),
                Block::Paragraph => paragraph.push_str(&t),
            },
            Event::Code(c) => paragraph.push_str(&c),
            Event::SoftBreak | Event::HardBreak => paragraph.push('\n'),
            _ => {}
        }
    }
    flush_paragraph(ui, &mut paragraph);
}

fn heading_size(level: u8) -> f32 {
    match level {
        1 => 26.0,
        2 => 22.0,
        3 => 19.0,
        _ => 17.0,
    }
}

fn code_lang(kind: &CodeBlockKind<'_>) -> String {
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

fn flush_paragraph(ui: &mut Ui, paragraph: &mut String) {
    if paragraph.trim().is_empty() {
        paragraph.clear();
        return;
    }
    ui.label(RichText::new(paragraph.trim()).size(15.0));
    ui.add_space(2.0);
    paragraph.clear();
}

fn render_code_block(
    ui: &mut Ui,
    code: &str,
    lang: &str,
    ss: &SyntaxSet,
    theme: &syntect::highlighting::Theme,
) {
    let job = code_block_job(code.trim_start_matches('\n'), lang, ss, theme);
    ui.label(job);
    ui.add_space(6.0);
}

/// Receta syntect → egui: resaltado token a token con colores del tema.
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
            let color = Color32::from_rgb(
                style.foreground.r,
                style.foreground.g,
                style.foreground.b,
            );
            job.append(span, 0.0, egui::TextFormat::simple(mono.clone(), color));
        }
    }
    job
}
