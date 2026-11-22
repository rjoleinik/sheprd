use super::*;
use crate::{sidebar::layout_tests::fixture_window, traces::service};
use gpui::TestAppContext;
use sheprd_control_protocol::{ActivityPanel, protocol::app::TraceFilter};
use sheprd_traces::{Host, Store, redact::Redactor};

#[gpui::test]
fn returning_to_a_host_filter_refreshes_dirty_counts_without_another_service_update(
    cx: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private/traces.sqlite3");
    let logs = dir.path().join("sessions");
    std::fs::create_dir(&logs).unwrap();
    let log = logs.join("session.jsonl");
    let line = format!(
        "{}\n",
        serde_json::json!({
            "type":"user", "timestamp":chrono::Utc::now().to_rfc3339(),
            "message":{"content":"fixture prompt"}
        })
    );
    std::fs::write(&log, &line).unwrap();
    let host = Host {
        id: Host::id_of("fixture"),
        label: "fixture".into(),
        user: "me@fixture".into(),
        os: "test".into(),
    };
    let roots = [sheprd_traces::Root {
        agent: AgentKind::Claude,
        dir: logs,
    }];
    let mut store = Store::open(&path).unwrap();
    sheprd_traces::index(&mut store, &host, &roots, &Redactor::new(None), NOW).unwrap();
    let (service, publish) = service::Service::fixture(path, service::snapshot(&store).unwrap());
    cx.update(|cx| cx.set_global(service));
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.config.sidebar_style.activity_bar = true;
        view.activity.panel = ActivityPanel::Traces;
        view.poll_traces(cx);
        view.set_trace_filter(
            TraceFilter::Host {
                id: host.id.clone(),
            },
            cx,
        );
        view
    });
    cx.run_until_parked();
    let messages = |view: &crate::SheprdWindow| {
        view.traces
            .page
            .as_ref()
            .unwrap()
            .activity
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .iter()
            .map(|day| day.messages)
            .sum::<i64>()
    };
    assert_eq!(view.read_with(cx, |view, _| messages(view)), 1);
    view.update(cx, |view, _| {
        view.activity.panel = ActivityPanel::Workspaces
    });
    std::fs::write(&log, line.repeat(2)).unwrap();
    sheprd_traces::index(&mut store, &host, &roots, &Redactor::new(None), NOW).unwrap();
    publish(service::snapshot(&store).unwrap());
    view.update(cx, |view, cx| {
        view.poll_traces(cx);
        assert_eq!(messages(view), 1);
        assert!(view.traces.page.as_ref().unwrap().activity_dirty);
        view.activity.panel = ActivityPanel::Traces;
        view.poll_traces(cx);
        assert!(view.traces.page.as_ref().unwrap().activity_task.is_some());
        assert!(!view.traces.page.as_ref().unwrap().activity_dirty);
        // The same generation and another tick do not start or queue a second read.
        view.poll_traces(cx);
        assert!(!view.traces.page.as_ref().unwrap().activity_dirty);
    });
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| messages(view)), 2);
}
