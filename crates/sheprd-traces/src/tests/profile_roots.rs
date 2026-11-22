use super::*;
use std::{collections::BTreeMap, ffi::OsString};

#[test]
fn overrides_replace_defaults_and_keep_active_and_archived_codex_history() {
    let home = tempfile::tempdir().unwrap();
    let cwd = home.path().join("working");
    let variables: BTreeMap<_, OsString> = [
        (
            "CLAUDE_CONFIG_DIR",
            cwd.join("claude profile").into_os_string(),
        ),
        ("CODEX_HOME", "relative codex".into()),
        ("PI_CONFIG_DIR", ".custom-omp".into()),
    ]
    .into();
    let roots = crate::configured_roots(home.path(), &cwd, |name| variables.get(name).cloned());
    let claude = cwd.join("claude profile/projects");
    let codex = cwd.join("relative codex/sessions");
    let archived = cwd.join("relative codex/archived_sessions");
    let pi = home.path().join(".custom-omp/agent/sessions");
    for path in [&claude, &codex, &archived, &pi] {
        assert!(roots.iter().any(|root| &root.dir == path));
    }
    assert!(
        !roots
            .iter()
            .any(|root| root.dir == home.path().join(".claude/projects"))
    );
    assert_eq!(roots.iter().filter(|root| root.dir == pi).count(), 1);
    write(&claude.join("session.jsonl"), fixtures::CLAUDE);
    write(&codex.join("session.jsonl"), fixtures::CODEX);
    write(&pi.join("session.jsonl"), fixtures::PI);
    let mut store = Store::in_memory().unwrap();
    index(&mut store, &host(), &roots, &redactor(), NOW).unwrap();
    for agent in [AgentKind::Claude, AgentKind::Codex, AgentKind::Omp] {
        assert!(
            !store.sessions(Some(agent), 10).unwrap().is_empty(),
            "{agent:?}"
        );
    }
}

#[test]
fn direct_pi_directory_takes_precedence_and_empty_values_use_defaults() {
    let home = Path::new("/home/example");
    let cwd = Path::new("/work");
    let roots = crate::configured_roots(home, cwd, |name| match name {
        "PI_CODING_AGENT_DIR" => Some("direct".into()),
        "PI_CONFIG_DIR" => Some("ignored".into()),
        _ => None,
    });
    assert_eq!(
        roots
            .iter()
            .filter(|root| root.dir == cwd.join("direct/sessions"))
            .count(),
        1
    );
    assert!(
        !roots
            .iter()
            .any(|root| root.dir == home.join("ignored/agent/sessions"))
    );
    assert_eq!(
        crate::configured_roots(home, cwd, |_| Some(OsString::new())),
        crate::default_roots(home)
    );
}

#[cfg(unix)]
#[test]
fn profile_paths_need_not_be_utf8() {
    use std::os::unix::ffi::OsStringExt;
    let custom = OsString::from_vec(b"/profiles/claude-\xff".to_vec());
    let roots = crate::configured_roots(Path::new("/home/me"), Path::new("/work"), |name| {
        (name == "CLAUDE_CONFIG_DIR").then(|| custom.clone())
    });
    assert!(roots.iter().any(|root| root.agent == AgentKind::Claude
        && root.dir == std::path::PathBuf::from(&custom).join("projects")));
}
