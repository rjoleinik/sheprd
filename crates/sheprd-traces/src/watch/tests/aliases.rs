use super::*;

#[test]
fn canonical_file_notifications_keep_the_configured_cursor_path() {
    let configured = PathBuf::from("/var/example/sessions");
    let watched = PathBuf::from("/private/var/example/sessions");
    let roots = [Root {
        agent: sheprd_client::herdr::AgentKind::Claude,
        dir: configured.clone(),
    }];
    let aliases = [Alias {
        watched: watched.clone(),
        configured: configured.clone(),
    }];
    let mut paths = BTreeSet::new();
    assert_eq!(
        collect(
            Ok(notify::Event::new(notify::EventKind::Any).add_path(watched.join("session.jsonl"))),
            &roots,
            &aliases,
            &mut paths
        ),
        Seen::Files
    );
    assert_eq!(paths, BTreeSet::from([configured.join("session.jsonl")]));
}

#[test]
fn canonical_parent_notifications_notice_an_agent_directory_created_later() {
    let configured = PathBuf::from("/var/example");
    let watched = PathBuf::from("/private/var/example");
    let roots = [Root {
        agent: sheprd_client::herdr::AgentKind::Pi,
        dir: configured.join(".pi/agent/sessions"),
    }];
    let aliases = [Alias {
        watched: watched.clone(),
        configured,
    }];
    assert_eq!(
        collect(
            Ok(notify::Event::new(notify::EventKind::Any).add_path(watched.join(".pi"))),
            &roots,
            &aliases,
            &mut BTreeSet::new()
        ),
        Seen::Parent
    );
}

#[cfg(unix)]
#[test]
fn translating_a_watched_prefix_does_not_follow_links_inside_it() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    let alias = dir.path().join("alias");
    fs::create_dir_all(real.join("sessions")).unwrap();
    symlink(&real, &alias).unwrap();
    let root = Root {
        agent: sheprd_client::herdr::AgentKind::Claude,
        dir: alias.join("sessions"),
    };
    let mut aliases = Vec::new();
    remember_alias(&mut aliases, &root.dir);
    let outside = dir.path().join("outside.jsonl");
    fs::write(
        &outside,
        "{\"type\":\"user\",\"message\":{\"content\":\"outside\"}}\n",
    )
    .unwrap();
    symlink(&outside, real.join("sessions/linked.jsonl")).unwrap();
    let notified = real
        .join("sessions")
        .canonicalize()
        .unwrap()
        .join("linked.jsonl");
    let mut paths = BTreeSet::new();
    collect(
        Ok(notify::Event::new(notify::EventKind::Any).add_path(notified)),
        std::slice::from_ref(&root),
        &aliases,
        &mut paths,
    );
    assert_eq!(paths, BTreeSet::from([root.dir.join("linked.jsonl")]));
    let host = Host {
        id: Host::id_of("aliases"),
        label: "box".into(),
        user: "me@box".into(),
        os: "test".into(),
    };
    let mut store = Store::in_memory().unwrap();
    index_paths(
        &mut store,
        &host,
        &[root],
        &paths.into_iter().collect::<Vec<_>>(),
        &Redactor::new(None),
    )
    .unwrap();
    assert!(store.sessions(None, 10).unwrap().is_empty());
}
