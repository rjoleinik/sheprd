#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::native::project::{Error, run};
use sheprd_client::filesystem::rename_noreplace;
use std::time::{Duration, Instant};

#[test]
fn publishing_refuses_an_empty_directory_created_after_the_vacancy_check() {
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("staged");
    std::fs::create_dir(&work).unwrap();
    std::fs::write(work.join("kept"), "project").unwrap();
    let target = dir.path().join("project");
    assert!(!target.exists());
    std::fs::create_dir(&target).unwrap();
    assert!(rename_noreplace(&work, &target).is_err());
    assert_eq!(std::fs::read_dir(&target).unwrap().count(), 0);
    assert_eq!(
        std::fs::read_to_string(work.join("kept")).unwrap(),
        "project"
    );
    std::fs::remove_dir(&target).unwrap();
    rename_noreplace(&work, &target).unwrap();
    assert!(!work.exists());
    assert_eq!(
        std::fs::read_to_string(target.join("kept")).unwrap(),
        "project"
    );
}

#[cfg(unix)]
#[test]
fn publishing_refuses_a_dangling_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("staged");
    std::fs::create_dir(&work).unwrap();
    let target = dir.path().join("project");
    std::os::unix::fs::symlink("missing", &target).unwrap();
    assert!(rename_noreplace(&work, &target).is_err());
    assert_eq!(
        std::fs::read_link(target).unwrap(),
        std::path::Path::new("missing")
    );
    assert!(work.is_dir());
}

#[test]
fn an_exited_parent_cannot_leave_helpers_holding_stderr() {
    #[cfg(unix)]
    let command = {
        let mut command = sheprd_client::new_command("/bin/sh");
        command.args(["-c", "sleep 30 >&2 & exit 1"]);
        command
    };
    #[cfg(windows)]
    let command = {
        use std::os::windows::process::CommandExt as _;
        let mut command = sheprd_client::new_command("cmd.exe");
        command.raw_arg(
            "/D /S /C \"start /B cmd.exe /D /S /C \"ping -n 31 127.0.0.1 >&2\" & exit /B 1\"",
        );
        command
    };
    let start = Instant::now();
    assert!(matches!(
        run(command, Duration::from_millis(200)),
        Err(Error::TimedOut)
    ));
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[cfg(unix)]
#[test]
fn continuous_output_cannot_extend_the_absolute_deadline() {
    let mut command = sheprd_client::new_command("/bin/sh");
    command.args(["-c", "while :; do printf progress >&2; done"]);
    let start = Instant::now();
    assert!(matches!(
        run(command, Duration::from_millis(200)),
        Err(Error::TimedOut)
    ));
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[cfg(unix)]
#[test]
fn command_errors_keep_their_diagnostics() {
    let mut command = sheprd_client::new_command("/bin/sh");
    command.args(["-c", "printf 'fatal: refused' >&2; exit 1"]);
    assert!(
        matches!(run(command, Duration::from_secs(2)), Err(Error::Refused { output }) if output == "fatal: refused")
    );
}
