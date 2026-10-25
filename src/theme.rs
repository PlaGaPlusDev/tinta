//! Tema visual: egui 0.37 usa API por tema (Theme/ThemePreference).

use egui::{Context, Theme, ThemePreference};

/// Activa tema claro u oscuro.
pub fn configure(ctx: &Context, dark: bool) {
    ctx.set_theme(if dark {
        ThemePreference::Dark
    } else {
        ThemePreference::Light
    });
    for theme in [Theme::Dark, Theme::Light] {
        ctx.style_mut_of(theme, |style| {
            if let Some(font) = style.text_styles.get_mut(&egui::TextStyle::Body) {
                font.size = 15.0;
            }
            style.interaction.selectable_labels = false;
        });
    }
}
