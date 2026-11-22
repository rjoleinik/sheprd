//! The name of the terminals no tab holds, beside the workspaces kept.

use super::*;

fn unlisted_label(workspaces: Value) -> String {
    let saved = serde_json::from_value(json!({"workspaces": workspaces, "tabs": []})).unwrap();
    let records = [serde_json::from_value(record("t1")).unwrap()];
    let layout = projection::project("fake-core", &records, Some(&saved), Some("s"));
    layout.workspace(UNLISTED).unwrap().label.clone()
}

#[test]
fn unlisted_terminals_are_not_named_like_the_default_workspace() {
    let label = unlisted_label(json!([
        {"id": "herdr-2", "scope": "s", "label": "Herdr 2", "cwd": ""}
    ]));
    assert_eq!(label, "Other terminals");
}

#[test]
fn unlisted_terminals_keep_their_name_with_no_namesake() {
    let label = unlisted_label(json!([
        {"id": "space-1", "scope": "s", "label": "api", "cwd": "/code/api"}
    ]));
    assert_eq!(label, "Herdr 2");
}

#[test]
fn the_stand_in_shell_is_listed_in_the_default_workspace() {
    let saved = serde_json::from_value(json!({"tabs": [], "workspaces": [
        {"id": sheprd_session::DEFAULT_WORKSPACE, "scope": "s", "label": "Herdr 2", "cwd": ""}
    ]}))
    .unwrap();
    let mut shell = record("t1");
    shell["name"] = json!(sheprd_session::STAND_IN_SHELL);
    let records = [shell, record("t2")].map(|record| serde_json::from_value(record).unwrap());
    let layout = projection::project("fake-core", &records, Some(&saved), Some("s"));
    let default = layout.workspace(sheprd_session::DEFAULT_WORKSPACE).unwrap();
    assert_eq!(default.terminals.len(), 1);
    assert_eq!(
        default.terminals[0].panes[0].label.as_deref(),
        Some("herdr-ui")
    );
    assert_eq!(layout.workspace(UNLISTED).unwrap().terminals.len(), 1);
}

#[test]
fn the_stand_in_shell_stays_unlisted_without_the_default_workspace() {
    let mut shell = record("t1");
    shell["name"] = json!(sheprd_session::STAND_IN_SHELL);
    let records = [serde_json::from_value(shell).unwrap()];
    let layout = projection::project("fake-core", &records, None, None);
    assert_eq!(layout.workspace(UNLISTED).unwrap().terminals.len(), 1);
    assert!(
        layout
            .workspace(sheprd_session::DEFAULT_WORKSPACE)
            .is_none()
    );
}
