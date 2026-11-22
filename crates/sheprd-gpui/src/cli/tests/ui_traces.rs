use super::*;
use control::{ControlCommand, Invocation, UiCommand};

#[test]
fn ui_traces_keeps_json_and_window_targeting_and_refuses_extra_arguments() {
    let parsed = LaunchOptions::parse(["ui", "traces", "--json", "--window", "42"]).unwrap();
    assert_eq!(
        parsed.mode,
        LaunchMode::Control(Invocation {
            command: ControlCommand::Ui(UiCommand::Traces),
            json: true,
            window: Some(42),
        })
    );
    assert!(LaunchOptions::parse(["ui", "traces", "unexpected"]).is_err());
}

#[test]
fn ui_traces_selects_a_host_or_label_and_can_clear_the_scope() {
    use sheprd_control_protocol::protocol::app::TraceFilter;
    for (args, filter) in [
        (
            vec!["ui", "traces", "--host", "host-id"],
            TraceFilter::Host {
                id: "host-id".into(),
            },
        ),
        (
            vec!["ui", "traces", "--tag", "personal"],
            TraceFilter::Tag {
                tag: "personal".into(),
            },
        ),
        (vec!["ui", "traces", "--all"], TraceFilter::All),
    ] {
        assert!(
            matches!(LaunchOptions::parse(args).unwrap().mode, LaunchMode::Control(Invocation { command: ControlCommand::Ui(UiCommand::TracesFilter(found)), .. }) if found == filter)
        );
    }
    for args in [
        vec!["ui", "traces", "--host", "x", "--tag", "work"],
        vec!["ui", "traces", "--tag", "work", "--all"],
        vec!["ui", "traces", "--host"],
        vec!["ui", "traces", "--tag", ""],
    ] {
        assert!(LaunchOptions::parse(args).is_err());
    }
}
