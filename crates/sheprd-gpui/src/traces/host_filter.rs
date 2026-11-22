//! Host identities and the existing device tags, independent of online state.
use super::*;
use crate::orchestrator::Look;
use sheprd_control_protocol::protocol::app::{TaggedHost, TraceFilter};
use std::{collections::BTreeMap, path::PathBuf};

mod publish;
pub(super) use publish::{Publisher, wanted};

pub(super) fn tags(hosts: &[TaggedHost]) -> Vec<String> {
    let mut tags = BTreeMap::new();
    for tag in hosts.iter().flat_map(|host| &host.tags.tags) {
        tags.entry(tag.to_lowercase())
            .or_insert_with(|| tag.clone());
    }
    tags.into_values().collect()
}

impl SheprdWindow {
    /// SQL aggregates a selected host set into at most one year of daily
    /// agent totals. Only one read runs per window; rapid edits retain the
    /// latest selection rather than queueing a query for every click.
    pub(super) fn refresh_trace_activity(&mut self, cx: &mut Context<Self>) {
        let Some(service) = cx.try_global::<Service>() else {
            return;
        };
        let path = service.index_path().map(PathBuf::from);
        if self.activity_section() != Some(ActivityPanel::Traces) {
            if let Some(page) = &mut self.traces.page {
                page.activity_dirty = true;
            }
            return;
        }
        let hosts = self.trace_hosts().to_vec();
        let Some(page) = &mut self.traces.page else {
            return;
        };
        if matches!(page.filter.scope, TraceFilter::All) {
            page.activity_dirty = false;
            return;
        }
        if page.activity_task.is_some() {
            page.activity_dirty = true;
            return;
        }
        let scope = page.filter.scope.clone();
        let ids: Vec<String> = hosts
            .iter()
            .filter(|host| scope.includes(&host.id, &hosts))
            .map(|host| host.id.clone())
            .collect();
        page.activity_dirty = false;
        let read = cx.background_executor().spawn(async move {
            let path = path.ok_or(sheprd_traces::Error::NoStateDirectory)?;
            let store = sheprd_traces::Store::open(&path)?;
            let until = chrono::Utc::now().timestamp_millis();
            let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
            store.activity_for(
                until - service::HISTORY_DAYS * 86_400_000,
                until,
                Some(&ids),
            )
        });
        page.activity_task = Some(cx.spawn(async move |this, cx| {
            let result = read.await.map_err(std::sync::Arc::new);
            let _ = this.update(cx, |view, cx| {
                let Some(page) = &mut view.traces.page else {
                    return;
                };
                let dirty = page.activity_dirty;
                if page.filter.scope == scope && !dirty {
                    page.activity = Some(result);
                }
                page.activity_task = None;
                if dirty {
                    view.refresh_trace_activity(cx);
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn trace_host_filters(
        &self,
        look: &Look,
        hosts: &[TaggedHost],
        filter: &TraceFilter,
        cx: &mut Context<Self>,
    ) -> Div {
        let choices = tags(hosts);
        let labels = choices.into_iter().map(|tag| {
            let scope = TraceFilter::Tag { tag: tag.clone() };
            let on = matches!(filter, TraceFilter::Tag { tag: selected } if sheprd_client::device_tags::same(selected, &tag));
            let selector = format!("traces-tag-{tag}");
            look.pill(selector.clone(), on)
                .debug_selector(move || selector.clone())
                .flex_none()
                .child(tag)
                .on_click(cx.listener(move |view, _, _, cx| view.set_trace_filter(scope.clone(), cx)))
        });
        let computers = hosts.iter().map(|host| {
            let scope = TraceFilter::Host {
                id: host.id.clone(),
            };
            let label = if hosts
                .iter()
                .any(|other| other.id != host.id && other.label == host.label)
            {
                fl!(
                    "traces-host-disambiguated",
                    host = host.label.as_str(),
                    id = host.id.chars().take(8).collect::<String>()
                )
            } else {
                host.label.clone()
            };
            let selector = format!("traces-host-{}", host.id);
            look.pill(selector.clone(), filter == &scope)
                .debug_selector(move || selector.clone())
                .flex_none()
                .max_w(px(200.))
                .truncate()
                .child(label)
                .on_click(
                    cx.listener(move |view, _, _, cx| view.set_trace_filter(scope.clone(), cx)),
                )
        });
        div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .id("traces-label-filters")
                    .overflow_x_scroll()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        look.pill("traces-all-hosts", matches!(filter, TraceFilter::All))
                            .debug_selector(|| "traces-all-hosts".into())
                            .flex_none()
                            .child(fl!("traces-all-hosts"))
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.set_trace_filter(TraceFilter::All, cx)
                            })),
                    )
                    .when(!hosts.is_empty(), |row| {
                        row.child(
                            div()
                                .flex_none()
                                .text_color(rgb(look.theme.muted))
                                .child(fl!("traces-labels")),
                        )
                    })
                    .children(labels),
            )
            .child(
                div()
                    .id("traces-host-filters")
                    .overflow_x_scroll()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_none()
                            .text_color(rgb(look.theme.muted))
                            .child(fl!("traces-hosts")),
                    )
                    .children(computers),
            )
    }
}
