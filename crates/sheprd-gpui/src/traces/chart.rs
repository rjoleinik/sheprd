//! Messages per day, one bar a day stacked by agent, scaled to the busiest
//! day, with the agents' colours as the filter shows them.

use super::{
    model::{Chart, Range},
    view::{agent_color, agent_label},
};
use crate::i18n::fl;
use crate::{config::corners, orchestrator::Look};
use gpui::{prelude::*, *};

const HEIGHT: f32 = 140.;
/// Space between bars while they are wide enough to show it.
const GAP: f32 = 2.;

pub(super) fn messages(look: &Look, chart: &Chart, range: Range) -> Div {
    let theme = &look.theme;
    let colors: Vec<u32> = chart
        .agents
        .iter()
        .map(|agent| agent_color(look, *agent))
        .collect();
    let legend = chart.agents.iter().zip(&colors).map(|(agent, color)| {
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(look.dot(*color, 7.))
            .child(agent_label(*agent))
    });
    let axis = {
        let label = |index: usize| {
            chart
                .days
                .get(index)
                .map(|day| day.format("%b %-d").to_string())
                .unwrap_or_default()
        };
        let last = chart.days.len().saturating_sub(1);
        div()
            .flex()
            .justify_between()
            .text_size(look.small())
            .text_color(rgb(theme.muted))
            .child(label(0))
            .child(label(last / 2))
            .child(label(last))
    };
    div()
        .debug_selector(|| "traces-chart".into())
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
                .flex_wrap()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(fl!("traces-chart-title")),
                )
                .child(
                    div()
                        .text_size(look.small())
                        .text_color(rgb(theme.subtext()))
                        .child(fl!(
                            "traces-chart-summary",
                            messages = chart.total,
                            days = range.days()
                        )),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_3()
                        .text_size(look.small())
                        .text_color(rgb(theme.subtext()))
                        .children(legend),
                ),
        )
        .child(bars(chart.counts.clone(), chart.peak, colors, theme.active))
        .child(axis)
}

fn bars(counts: Vec<Vec<u32>>, peak: u32, colors: Vec<u32>, baseline: u32) -> AnyElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            window.paint_quad(fill(
                Bounds::new(
                    point(bounds.origin.x, bounds.bottom() - px(1.)),
                    size(bounds.size.width, px(1.)),
                ),
                rgb(baseline),
            ));
            let scale = f32::from(bounds.size.height) / peak.max(1) as f32;
            let slot = f32::from(bounds.size.width) / counts.len().max(1) as f32;
            let gap = if slot > 3. * GAP { GAP } else { 0. };
            let width = px((slot - gap).max(0.5));
            for (index, day) in counts.iter().enumerate() {
                let left = bounds.origin.x + px(index as f32 * slot);
                let mut top = bounds.bottom();
                for (count, color) in day.iter().zip(&colors) {
                    if *count == 0 {
                        continue;
                    }
                    let height = px(*count as f32 * scale);
                    top -= height;
                    window.paint_quad(fill(
                        Bounds::new(point(left, top), size(width, height)),
                        rgb(*color),
                    ));
                }
            }
        },
    )
    .w_full()
    .h(px(HEIGHT))
    .into_any_element()
}
