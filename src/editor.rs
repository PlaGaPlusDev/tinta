//! Render de Markdown a widgets egui en modo lectura.
//! Bloques de código con resaltado syntect → LayoutJob.

use egui::{Color32, FontId, RichText, Ui};
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use std::sync::OnceLock;

// syntect carga sus gramáticas una sola vez; el modo inmediato lo reutiliza.
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

/// Renderiza un texto Markdown en modo lectura.
pub fn render_markdown(ui: &mut Ui, source: &str) {
    let ss = syntax_set();
    let theme = code_theme();
    let mut state = Block::Paragraph;
    let mut paragraph = String::new();
    let mut heading_buf = String::new();
    let mut heading_level = 1u8;
    let mut code_buf = String::new();
    let mut code_lang = String::new();

    for event in Parser::new(source) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_paragraph(ui, &mut paragraph);
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
                flush_paragraph(ui, &mut paragraph);
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

fn flush_paragraph(ui: &mut Ui, paragraph: &mut String) {
    if paragraph.trim().is_empty() {
        paragraph.clear();
        return;
    }
    ui.label(RichText::new(paragraph.trim()).size(15.0));
    ui.add_space(2.0);
    paragraph.clear();
}

/// Resaltado syntect token a token → colores estilo túnel a egui.
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
