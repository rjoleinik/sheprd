//! What the phone's styles read: the step without its shell wrapper, the
//! step in words, the steps before it in words, and the turn's counts.

use super::*;
use crate::apns::payload::Icon;

fn edit(id: &str, path: &str) -> EntryKind {
    EntryKind::Tool(Tool {
        call_id: id.to_owned(),
        content: Vec::new(),
        kind: ToolKind::Edit,
        locations: vec![path.to_owned()],
        parent: None,
        started_ms: None,
        status: ToolStatus::Completed,
        title: format!("Edit {path}"),
    })
}

#[test]
fn a_command_step_is_cleaned_and_told_in_words() {
    let mut before = running();
    before.entries.push(Entry {
        id: EntryId(2),
        kind: tool("`/bin/zsh -lc 'git status --short'`", ToolStatus::Completed),
    });
    before.entries.push(Entry {
        id: EntryId(3),
        kind: edit("e1", "/w/a.rs"),
    });
    before.entries.push(Entry {
        id: EntryId(4),
        kind: edit("e2", "/w/a.rs"),
    });
    before.entries.push(Entry {
        id: EntryId(5),
        kind: edit("e3", "/w/b.rs"),
    });
    let change = push(
        6,
        tool(
            "`/bin/zsh -lc \"gh pr checks 1548\"`",
            ToolStatus::InProgress,
        ),
    );
    let live = decide(&before, &change).unwrap();
    let state = &live.state;
    assert_eq!(state.step, "gh pr checks 1548");
    assert_eq!(
        state.detail.activity.as_deref(),
        Some("Checking pull request #1548")
    );
    assert_eq!(state.detail.icon, Some(Icon::Github));
    // The last two other steps, oldest first.
    assert_eq!(state.previous, ["Edit /w/a.rs", "Edit /w/b.rs"]);
    assert_eq!(
        state.detail.previous_activities,
        ["Editing a.rs", "Editing b.rs"]
    );
    // Four calls before, and this new one.
    assert_eq!(state.detail.steps, Some(5));
    assert_eq!(state.detail.files, Some(2), "a.rs counts once");
}

#[test]
fn a_plan_step_is_its_own_words() {
    let mut before = running();
    before.plan = plan(&[("Split the tests", PlanStatus::Pending)]);
    let change = Change::Plan {
        plan: plan(&[("Split the tests", PlanStatus::InProgress)]),
    };
    let live = decide(&before, &change).unwrap();
    assert_eq!(live.state.step, "Split the tests");
    assert_eq!(
        live.state.detail.activity.as_deref(),
        Some("Split the tests")
    );
    assert_eq!(live.state.detail.icon, Some(Icon::Plan));
    assert_eq!(live.state.detail.steps, None, "no tool call yet");
}

#[test]
fn a_new_turn_starts_with_no_detail() {
    let change = Change::Turn {
        turn: Some(Turn {
            started_ms: STARTED_MS,
        }),
    };
    let live = decide(&idle(), &change).unwrap();
    assert_eq!(live.state.detail, Detail::default());
}

/// A command run again reads the same, but it is a new call: the counts
/// move, so it is sent. The same call again is not.
#[test]
fn a_repeated_command_still_counts() {
    let mut before = running();
    before.entries.push(Entry {
        id: EntryId(2),
        kind: tool("cargo test", ToolStatus::InProgress),
    });
    let again = |id: &str| {
        let EntryKind::Tool(mut call) = tool("cargo test", ToolStatus::InProgress) else {
            unreachable!()
        };
        call.call_id = id.to_owned();
        push(3, EntryKind::Tool(call))
    };
    let live = decide(&before, &again("second")).unwrap();
    assert_eq!(live.state.step, "cargo test");
    assert_eq!(live.state.detail.steps, Some(2));
    assert!(
        decide(&before, &again("cargo test")).is_none(),
        "the same call again"
    );
}
