use super::*;
use crate::sidebar::layout_tests::fixture_window;

#[gpui::test]
fn traces_status_reads_prepared_state_without_a_worker(cx: &mut gpui::TestAppContext) {
    let (_, cx) = cx.add_window_view(fixture_window);
    let response = cx.cx.update(|cx| ui(call(UiOp::Traces {}), cx));
    assert!(
        matches!(response, Response::App { reply: Reply::Traces { peers, .. } } if peers.is_empty())
    );
    let mut request = call(UiOp::Traces {});
    request.caller.window = Some(u64::MAX);
    let refused = cx.cx.update(|cx| ui(request, cx));
    assert_eq!(code(&refused), Some(ErrorCode::NotFound));
}

#[gpui::test]
fn traces_filters_use_indexed_hosts_and_existing_labels_even_offline(
    cx: &mut gpui::TestAppContext,
) {
    use crate::control::protocol::app::{TaggedHost, TraceFilter};
    use crate::traces::service::{Snapshot, Status};
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.config.sidebar_style.activity_bar = true;
        view.traces.status = Status::Ready(std::sync::Arc::new(Snapshot {
            hosts: vec![TaggedHost {
                id: "host-id".into(),
                label: "box".into(),
                tags: sheprd_client::device_tags::ComputerTags::new(["work"], 1),
            }],
            ..Snapshot::default()
        }));
        view
    });
    for filter in [
        TraceFilter::Host {
            id: "host-id".into(),
        },
        TraceFilter::Tag { tag: "WORK".into() },
        TraceFilter::All,
    ] {
        let reply = cx.cx.update(|cx| {
            ui(
                call(UiOp::TracesFilter {
                    filter: filter.clone(),
                }),
                cx,
            )
        });
        assert!(
            matches!(reply, Response::App { reply: Reply::Traces { filter: found, .. } } if found == filter)
        );
        assert_eq!(view.read_with(cx, |view, _| view.trace_filter()), filter);
    }
    let refused = cx.cx.update(|cx| {
        ui(
            call(UiOp::TracesFilter {
                filter: TraceFilter::Host {
                    id: "missing".into(),
                },
            }),
            cx,
        )
    });
    assert_eq!(code(&refused), Some(ErrorCode::NotFound));
    assert_eq!(
        view.read_with(cx, |view, _| view.trace_filter()),
        TraceFilter::All
    );
}
