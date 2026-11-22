use super::*;

#[test]
fn pi_session_is_indexed_with_its_name_and_all_usage() {
    let home = tempfile::tempdir().unwrap();
    let path = home
        .path()
        .join(".pi/agent/sessions/--home-me-code-site--/2026-11-17T09-00-00_7f3a-pi.jsonl");
    write(&path, fixtures::PI);
    let mut store = Store::in_memory().unwrap();
    index(&mut store, &host(), &roots(home.path()), &redactor(), NOW).unwrap();

    let sessions = store.sessions(Some(AgentKind::Pi), 10).unwrap();
    let [session] = sessions.as_slice() else {
        panic!("one session: {sessions:?}");
    };
    assert_eq!(session.native_id, "7f3a-pi");
    // A name set later wins over the first prompt.
    assert_eq!(session.title.as_deref(), Some("Sticky header"));
    assert_eq!(session.model.as_deref(), Some("gpt-5.5"));
    assert_eq!(session.cwd.as_deref(), Some("/home/me/code/site"));
    // Assistant usage plus the cache-warming usage entry.
    assert_eq!(
        (session.tokens_in, session.tokens_out),
        (225 + 1 + 1000, 34)
    );
    assert_eq!(
        (session.messages, session.tool_calls, session.errors),
        (2, 1, 1)
    );
    let kinds: Vec<_> = store
        .events(&session.id, 0, 100)
        .unwrap()
        .iter()
        .map(|event| (event.kind, event.tool.clone()))
        .collect();
    assert_eq!(
        kinds,
        [
            (EventKind::User, None),
            (EventKind::Thinking, None),
            (EventKind::ToolCall, Some("edit".into())),
            (EventKind::ToolResult, Some("edit".into())),
            (EventKind::Agent, None),
            (EventKind::Error, None),
        ]
    );
}

#[test]
fn omp_sessions_are_read_as_pi_sessions() {
    let home = tempfile::tempdir().unwrap();
    write(
        &home
            .path()
            .join(".omp/agent/sessions/--home-me--/2026-11-17T09-00-00_7f3a-pi.jsonl"),
        fixtures::PI,
    );
    let mut store = Store::in_memory().unwrap();
    index(&mut store, &host(), &roots(home.path()), &redactor(), NOW).unwrap();
    let sessions = store.sessions(Some(AgentKind::Omp), 10).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].messages, 2);
}
