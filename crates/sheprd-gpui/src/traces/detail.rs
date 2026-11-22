//! One session beside the table: what it was, where, and its events in
//! order, each a kind, an optional tool, and its (redacted, shortened) text.

use super::{
    Detail, Page,
    view::{agent_color, agent_label, note, title, when},
};
use crate::i18n::fl;
use crate::{SheprdWindow, config::corners, fonts::StyledFont, orchestrator::Look};
use gpui::{prelude::*, *};
use sheprd_traces::{Event, EventKind, Session};

/// The longest event text drawn; the index keeps up to 4 KB.
const SHOWN_TEXT: usize = 600;
/// The events column's own scroll height.
const EVENTS_HEIGHT: f32 = 560.;

pub(super) fn pane(
    look: &Look,
    session: &Session,
    page: &Page,
    now: chrono::DateTime<chrono::Utc>,
    cx: &mut Context<SheprdWindow>,
) -> Div {
    let theme = &look.theme;
    let session_id = session.id.clone();
    let facts = [
        (fl!("traces-fact-agent"), agent_label(session.agent)),
        (fl!("traces-fact-host"), session.host.clone()),
        (
            fl!("traces-fact-model"),
            session.model.clone().unwrap_or_default(),
        ),
        (
            fl!("traces-fact-folder"),
            session.cwd.clone().unwrap_or_default(),
        ),
        (
            fl!("traces-fact-branch"),
            session.branch.clone().unwrap_or_default(),
        ),
        (fl!("traces-fact-started"), when(session.started_at, now)),
        (
            fl!("traces-fact-tokens"),
            fl!(
                "traces-fact-tokens-value",
                input = super::model::compact(session.tokens_in),
                output = super::model::compact(session.tokens_out)
            ),
        ),
    ];
    let events: AnyElement = match &page.detail {
        None | Some(Detail::Loading) => note(look, fl!("traces-events-loading")).into_any_element(),
        Some(Detail::Failed(error)) => {
            note(look, fl!("traces-events-failed", error = error.as_str())).into_any_element()
        }
        Some(Detail::Loaded(events)) if events.is_empty() => {
            note(look, fl!("traces-events-none")).into_any_element()
        }
        Some(Detail::Loaded(events)) => div()
            .id("traces-events")
            .max_h(px(EVENTS_HEIGHT))
            .overflow_y_scroll()
            .track_scroll(&page.detail_scroll)
            .flex()
            .flex_col()
            .gap_2()
            .children(events.iter().map(|event| event_row(look, event)))
            .into_any_element(),
    };
    div()
        .debug_selector(|| "traces-detail".into())
        .rounded(px(corners::PANEL))
        .border_1()
        .border_color(rgb(theme.active))
        .bg(rgb(theme.surface))
        .p_4()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .items_start()
                .gap_2()
                .child(look.dot(agent_color(look, session.agent), 9.).mt(px(5.)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title(session)),
                )
                .child(
                    look.button("traces-detail-close", fl!("traces-close"), false)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.select_trace(session_id.clone(), cx);
                        })),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .text_size(look.small())
                .children(
                    facts
                        .into_iter()
                        .filter(|(_, value)| !value.is_empty())
                        .map(|(label, value)| {
                            div()
                                .flex()
                                .gap_2()
                                .child(
                                    div()
                                        .flex_none()
                                        .w(px(64.))
                                        .text_color(rgb(theme.muted))
                                        .child(label),
                                )
                                .child(div().min_w_0().truncate().child(value))
                        }),
                ),
        )
        .child(div().h(px(1.)).bg(rgb(theme.active)))
        .child(events)
}

fn event_row(look: &Look, event: &Event) -> Div {
    let theme = &look.theme;
    let (label, color) = match event.kind {
        EventKind::User => (fl!("traces-event-user"), look.hue(4)),
        EventKind::Agent => (fl!("traces-event-agent"), look.hue(2)),
        EventKind::Thinking => (fl!("traces-event-thinking"), theme.muted),
        EventKind::ToolCall => (fl!("traces-event-tool"), look.hue(6)),
        EventKind::ToolResult => (fl!("traces-event-result"), theme.subtext()),
        EventKind::Error => (fl!("traces-event-error"), look.hue(1)),
    };
    let text = shorten(&event.text);
    let quiet = matches!(event.kind, EventKind::Thinking | EventKind::ToolResult);
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(look.tag(label, color))
                .when_some(event.tool.clone(), |row, tool| {
                    row.child(look.mono(tool).text_color(rgb(theme.subtext())))
                }),
        )
        .when(!text.is_empty(), |row| {
            row.child(
                div()
                    .text_size(look.small())
                    .when(quiet, |el| el.text_color(rgb(theme.subtext())))
                    .when(event.kind == EventKind::ToolCall, |el| {
                        el.text_font(&look.mono)
                    })
                    .child(text),
            )
        })
}

/// At most [`SHOWN_TEXT`] bytes, cut on a character boundary.
fn shorten(text: &str) -> String {
    if text.len() <= SHOWN_TEXT {
        return text.to_owned();
    }
    let mut end = SHOWN_TEXT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}
