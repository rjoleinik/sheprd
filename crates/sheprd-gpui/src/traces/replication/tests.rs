#![allow(clippy::unwrap_used)]
use super::*;
use crate::{sidebar::layout_tests::fixture_window, state::ConnectionStatus};
use sheprd_client::ConnectTarget;

#[gpui::test]
fn replication_uses_connected_enabled_hosts_and_deduplicates_sessions(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, _| {
        let target = ConnectTarget::Ssh {
            target: "me@box".into(),
            session: "one".into(),
        };
        // The fixture's endpoint is selected, so the window's live status wins.
        view.endpoints[view.selected_endpoint].connection.target = target;
        view.endpoints[view.selected_endpoint].enabled = true;
        view.live.status = ConnectionStatus::Connected;
        let found = hosts(view);
        assert_eq!(found.len(), 1);
        assert!(found.contains_key("ssh:me@box"));
        view.endpoints[view.selected_endpoint].enabled = false;
        assert!(hosts(view).is_empty());
    });
}

#[test]
fn a_machine_key_is_independent_of_the_session() {
    let host = |session: &str| {
        Host::new(&ConnectTarget::Ssh {
            target: "me@box".into(),
            session: session.into(),
        })
        .unwrap()
    };
    assert_eq!(
        host("one").machine_identity(),
        host("two").machine_identity()
    );
    assert_ne!(host("one").identity(), host("two").identity());
}

#[test]
fn dropping_a_disconnected_job_cancels_its_transport() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let (_, result) = mpsc::sync_channel(1);
    let job = Job {
        cancelled: cancelled.clone(),
        result,
    };
    drop(job);
    assert!(cancelled.load(Ordering::Acquire));
}
