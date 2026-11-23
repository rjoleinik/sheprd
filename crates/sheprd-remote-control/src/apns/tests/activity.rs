//! A step in words: the shell wrapper taken off, and what a call does.

use crate::apns::activity::{Described, clean, describe, planned};
use crate::apns::payload::Icon;
use sheprd_chat_protocol::{Tool, ToolKind, ToolStatus};

fn tool(kind: ToolKind, title: &str, locations: &[&str]) -> Tool {
    Tool {
        call_id: "c".to_owned(),
        content: Vec::new(),
        kind,
        locations: locations.iter().map(|path| (*path).to_owned()).collect(),
        parent: None,
        started_ms: None,
        status: ToolStatus::InProgress,
        title: title.to_owned(),
    }
}

fn said(kind: ToolKind, title: &str) -> (String, Icon) {
    let Described { activity, icon } = describe(&tool(kind, title, &[]));
    (activity, icon)
}

#[test]
fn the_shell_wrapper_and_quotes_come_off() {
    for (title, command) in [
        (
            "`/bin/zsh -lc \"gh pr checks 1548 --repo sheprd-labs/sheprd\"`",
            "gh pr checks 1548 --repo sheprd-labs/sheprd",
        ),
        ("/bin/bash -c 'git status --short'", "git status --short"),
        ("sh -c ls", "ls"),
        ("`cargo test -p x`", "cargo test -p x"),
        ("cargo test -p x", "cargo test -p x"),
        // Not a wrapper: kept as it is.
        ("zsh scripts/run.sh", "zsh scripts/run.sh"),
        ("Edit src/main.rs", "Edit src/main.rs"),
    ] {
        assert_eq!(clean(title), command, "{title}");
    }
}

#[test]
fn commands_are_said_in_words() {
    for (title, activity, icon) in [
        (
            "`/bin/zsh -lc \"gh pr checks 1548 --repo sheprd-labs/sheprd\"`",
            "Checking pull request #1548",
            Icon::Github,
        ),
        ("gh pr view 12", "Reading pull request #12", Icon::Github),
        (
            "gh pr create --fill",
            "Opening a pull request",
            Icon::Github,
        ),
        (
            "gh api repos/a/b/issues/1/comments",
            "Asking GitHub",
            Icon::Github,
        ),
        ("git push origin feat/x", "Pushing", Icon::Git),
        (
            "git diff --cached --check",
            "Looking at the changes",
            Icon::Git,
        ),
        (
            "cargo test -p sheprd-remote-control",
            "Running tests",
            Icon::Test,
        ),
        ("just test-live /x", "Running tests", Icon::Test),
        ("just ci", "Running checks", Icon::Test),
        (
            "RUST_LOG=debug cargo build --release",
            "Building",
            Icon::Build,
        ),
        (
            "rg -n Live crates | head",
            "Searching the code",
            Icon::Search,
        ),
        ("sed -n 1,80p src/lib.rs", "Reading files", Icon::Read),
        (
            "curl -s https://example.org",
            "Fetching from the web",
            Icon::Web,
        ),
        ("./scripts/release.sh", "Running a command", Icon::Run),
    ] {
        assert_eq!(
            said(ToolKind::Execute, title),
            (activity.to_owned(), icon),
            "{title}"
        );
    }
}

/// Of a sequence the last command counts; of a pipeline the first, and a
/// redirection is no separator.
#[test]
fn the_command_that_counts_is_found_in_a_line() {
    assert_eq!(
        said(ToolKind::Execute, "cd crates && cargo test").0,
        "Running tests"
    );
    assert_eq!(
        said(ToolKind::Execute, "git status --short; gh pr checks 9").0,
        "Checking pull request #9"
    );
    assert_eq!(
        said(ToolKind::Execute, "cargo build 2>&1 | tail -5").0,
        "Building"
    );
    assert_eq!(
        said(ToolKind::Execute, "make || true").0,
        "Running a command"
    );
}

#[test]
fn other_kinds_are_said_by_kind() {
    let edit = describe(&tool(ToolKind::Edit, "Edit", &["/w/src/apns/decide.rs"]));
    assert_eq!(
        edit,
        Described {
            activity: "Editing decide.rs".to_owned(),
            icon: Icon::Edit
        }
    );
    assert_eq!(
        describe(&tool(ToolKind::Read, "Read", &[])).activity,
        "Reading files"
    );
    assert_eq!(said(ToolKind::Search, "grep foo").0, "Searching the code");
    assert_eq!(said(ToolKind::Think, "Thinking").1, Icon::Think);
    assert_eq!(said(ToolKind::Subagent, "Explore").1, Icon::Agent);
    // A kindless call is a command when it is wrapped as one.
    assert_eq!(said(ToolKind::Other, "`bash -lc 'git push'`").0, "Pushing");
    assert_eq!(
        said(ToolKind::Other, "Update todo list"),
        ("Update todo list".to_owned(), Icon::Run)
    );
    assert_eq!(planned("Split the tests").icon, Icon::Plan);
}
