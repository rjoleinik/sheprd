use super::super::traces_text;
use sheprd_control_protocol::protocol::app::{TracePeer, TraceSyncState};

#[test]
fn traces_status_names_syncing_ready_and_failed_hosts() {
    let peers = vec![
        TracePeer {
            host: "one".into(),
            state: TraceSyncState::Syncing,
        },
        TracePeer {
            host: "two".into(),
            state: TraceSyncState::Ready,
        },
        TracePeer {
            host: "three".into(),
            state: TraceSyncState::Failed {
                error: "helper missing".into(),
            },
        },
    ];
    assert_eq!(
        traces_text(&peers),
        "one: synchronizing\ntwo: up to date at the last exchange\nthree: failed: helper missing"
    );
    assert_eq!(traces_text(&[]), "No connected hosts to synchronize traces");
}
