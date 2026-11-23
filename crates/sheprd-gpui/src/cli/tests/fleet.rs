use super::*;
use control::{ControlCommand, FleetCommand, Invocation};

fn parse(args: &[&str]) -> Result<Invocation, CliError> {
    LaunchOptions::parse(args.iter().copied()).map(|options| match options.mode {
        LaunchMode::Control(invocation) => invocation,
        mode => panic!("{args:?} parsed as {mode:?}"),
    })
}

fn command(args: &[&str]) -> ControlCommand {
    parse(args).unwrap().command
}

#[test]
fn fleet_reports_status_by_default() {
    for args in [&["fleet"][..], &["fleet", "status"]] {
        assert_eq!(command(args), ControlCommand::Fleet(FleetCommand::Status));
    }
    let json = parse(&["fleet", "hosts", "--json", "--window", "3"]).unwrap();
    assert!(json.json);
    assert_eq!(json.window, Some(3));
    assert_eq!(json.command, ControlCommand::Fleet(FleetCommand::Hosts));
    assert_eq!(
        command(&["fleet", "members"]),
        ControlCommand::Fleet(FleetCommand::Members)
    );
    assert_eq!(
        command(&["fleet", "open"]),
        ControlCommand::Fleet(FleetCommand::Open)
    );
}

#[test]
fn fleet_removals_name_a_member_or_host() {
    for verb in ["remove", "remove-member"] {
        assert_eq!(
            command(&["fleet", verb, "old", "laptop"]),
            ControlCommand::Fleet(FleetCommand::Remove {
                member: "old laptop".into()
            })
        );
    }
    assert_eq!(
        command(&["fleet", "remove-host", "build-01"]),
        ControlCommand::Fleet(FleetCommand::RemoveHost {
            host: "build-01".into()
        })
    );
    assert!(matches!(
        parse(&["fleet", "remove"]),
        Err(CliError::MissingArgument { .. })
    ));
    assert_eq!(
        command(&["fleet", "scrub", "me@build-01"]),
        ControlCommand::Fleet(FleetCommand::Scrub {
            machine: "me@build-01".into()
        })
    );
    assert!(parse(&["fleet", "found"]).is_err());
}

#[test]
fn fleet_invite_names_a_machine_or_all() {
    assert_eq!(
        command(&["fleet", "invite", "m4max"]),
        ControlCommand::Fleet(FleetCommand::Invite {
            machine: Some("m4max".into())
        })
    );
    assert_eq!(
        command(&["fleet", "invite", "--all"]),
        ControlCommand::Fleet(FleetCommand::Invite { machine: None })
    );
    // Neither, or both, says nothing clear.
    for args in [
        &["fleet", "invite"][..],
        &["fleet", "invite", "--all", "m4max"],
    ] {
        assert!(matches!(parse(args), Err(CliError::MissingArgument { .. })));
    }
}

#[test]
fn fleet_join_may_name_the_member_reached() {
    assert_eq!(
        command(&["fleet", "join"]),
        ControlCommand::Fleet(FleetCommand::Join { machine: None })
    );
    assert_eq!(
        command(&["fleet", "join", "studio"]),
        ControlCommand::Fleet(FleetCommand::Join {
            machine: Some("studio".into())
        })
    );
}
