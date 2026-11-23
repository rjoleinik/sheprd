use super::*;

pub(super) fn workspace_mark(
    key: &str,
    number: &str,
    repository: Option<&str>,
    font: &FontConfig,
    columns: Columns,
    theme: &Theme,
    focused: bool,
) -> Div {
    let color = if focused {
        theme.foreground
    } else {
        theme.muted
    };
    let Some(name) = repository else {
        return mark_slot(
            format!("rail-mark-{key}"),
            columns.badge,
            columns.height,
            color,
            number,
        );
    };
    let size = line_height(font).min(columns.height);
    div()
        .debug_selector(|| format!("rail-mark-{key}"))
        .flex_none()
        .w(px(columns.badge))
        .h(px(columns.height))
        .row()
        .items_center()
        .gap(px(2.))
        .child(
            div()
                .debug_selector(|| format!("rail-index-{key}"))
                .w(px((columns.badge - size - 2.).max(0.)))
                .text_size(px(font.size / 2.))
                .font_weight(FontWeight::NORMAL)
                .text_color(rgb(color))
                .overflow_hidden()
                .child(label_text(number)),
        )
        .child(
            div()
                .debug_selector(|| format!("rail-repository-{key}"))
                .size(px(size))
                .flex_none()
                .child(repository::Tile::new(name, theme).element(size)),
        )
}
