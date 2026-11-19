use super::*;
use control::ChangeCommand;

#[test]
fn change_log_takes_an_optional_commit_prefix() {
    for args in [&["change"][..], &["change", "log"]] {
        assert_eq!(
            command(args),
            ControlCommand::Change(ChangeCommand::Log { commit: None })
        );
    }
    assert_eq!(
        command(&["change", "log", "ABCD123", "--json"]),
        ControlCommand::Change(ChangeCommand::Log {
            commit: Some("abcd123".into())
        })
    );
    for bad in ["abc", "not-hex", &"a".repeat(65)] {
        assert_eq!(
            parse(&["change", "log", bad]).unwrap_err(),
            CliError::InvalidValue("COMMIT"),
            "{bad}"
        );
    }
    assert!(parse(&["change", "evolve"]).is_err());
}
