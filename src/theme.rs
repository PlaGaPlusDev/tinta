/// Tema visual oscuro por defecto y alternancia claro/oscuro.
pub fn configure(ctx: &egui::Context, dark: bool) {
    ctx.set_visuals(if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    });
    let style = ctx.style_mut(|s| {
        // Mono consistente para editor; respuesta directa para el resto.
        if let Some(body) = s.text_styles.get_mut(&egui::TextStyle::Body) {
            body.size = 15.0;
        }
        s.interaction.selectable_labels = false;
    });
    ctx.set_style(style);
}
