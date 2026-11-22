use super::*;

#[cfg(unix)]
use std::os::unix::fs::symlink as symlink_file;
#[cfg(unix)]
use std::os::unix::fs::symlink as symlink_dir;
#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};

#[cfg(any(unix, windows))]
#[test]
fn full_and_targeted_scans_refuse_linked_files_directories_and_roots() {
    let dir = tempfile::tempdir().unwrap();
    let outside = dir.path().join("outside");
    let root = dir.path().join("sessions");
    fs::create_dir(&root).unwrap();
    write(&outside.join("session.jsonl"), fixtures::CLAUDE);
    let linked = symlink_file(outside.join("session.jsonl"), root.join("linked.jsonl"));
    #[cfg(windows)]
    if let Err(error) = &linked
        && (error.kind() == std::io::ErrorKind::PermissionDenied
            || error.raw_os_error() == Some(1314))
    {
        eprintln!("symbolic-link privilege unavailable: {error}; junctions have their own test");
        return;
    }
    linked.unwrap();
    symlink_dir(&outside, root.join("linked-dir")).unwrap();
    let roots = [Root {
        agent: AgentKind::Claude,
        dir: root.clone(),
    }];
    let mut store = Store::in_memory().unwrap();
    index(&mut store, &host(), &roots, &redactor(), NOW).unwrap();
    crate::index_paths(
        &mut store,
        &host(),
        &roots,
        &[
            root.join("linked.jsonl"),
            root.join("linked-dir/session.jsonl"),
            root.join("../outside/session.jsonl"),
        ],
        &redactor(),
    )
    .unwrap();
    assert!(store.sessions(None, 10).unwrap().is_empty());
    let linked_root = dir.path().join("linked-root");
    symlink_dir(&outside, &linked_root).unwrap();
    let roots = [Root {
        agent: AgentKind::Claude,
        dir: linked_root.clone(),
    }];
    index(&mut store, &host(), &roots, &redactor(), NOW).unwrap();
    crate::index_paths(
        &mut store,
        &host(),
        &roots,
        &[linked_root.join("session.jsonl")],
        &redactor(),
    )
    .unwrap();
    assert!(store.sessions(None, 10).unwrap().is_empty());
}

#[test]
fn watcher_paths_with_parent_components_are_not_session_files() {
    let root = Path::new("sessions");
    assert!(!crate::scan::session_file(
        root,
        &root.join("../private/session.jsonl")
    ));
    assert!(crate::scan::session_file(
        root,
        &root.join("project/session.jsonl")
    ));
}

#[cfg(windows)]
#[test]
fn junctions_are_refused_without_requiring_symbolic_link_privilege() {
    let dir = tempfile::tempdir().unwrap();
    let outside = dir.path().join("outside");
    let root = dir.path().join("sessions");
    fs::create_dir(&root).unwrap();
    write(&outside.join("session.jsonl"), fixtures::CLAUDE);
    let linked = root.join("junction");
    let status = sheprd_client::new_command("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&linked)
        .arg(&outside)
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    let mut store = Store::in_memory().unwrap();
    for root in [root, linked.clone()] {
        let roots = [Root {
            agent: AgentKind::Claude,
            dir: root,
        }];
        index(&mut store, &host(), &roots, &redactor(), NOW).unwrap();
        crate::index_paths(
            &mut store,
            &host(),
            &roots,
            &[linked.join("session.jsonl")],
            &redactor(),
        )
        .unwrap();
    }
    assert!(store.sessions(None, 10).unwrap().is_empty());
}
