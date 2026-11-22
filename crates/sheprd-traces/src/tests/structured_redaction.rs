use super::*;
use serde_json::json;

#[test]
fn tool_objects_and_json_arguments_are_redacted_before_storage_and_replication() {
    let secret = json!({
        "command": "login", "env": {"PASSWORD": "hunter2"},
        "items": [{"api_key": "tiny-key"}],
        "credentials": {"user": "private-user", "value": "private-value"},
        "encoded": json!({"nested": {"password": "nested-secret"}}).to_string(),
    });
    let cases = [
        (
            AgentKind::Claude,
            json!({"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":secret}]}}),
        ),
        (
            AgentKind::Codex,
            json!({"type":"response_item","payload":{"type":"function_call","name":"shell","arguments":secret.to_string()}}),
        ),
        (
            AgentKind::Pi,
            json!({"type":"message","message":{"role":"assistant","content":[{"type":"toolCall","name":"shell","arguments":secret}]}}),
        ),
        (
            AgentKind::Omp,
            json!({"type":"message","message":{"role":"toolResult","toolName":"shell","content":secret}}),
        ),
    ];
    for (agent, record) in cases {
        let home = tempfile::tempdir().unwrap();
        write(&home.path().join("session.jsonl"), &format!("{record}\n"));
        let mut store = Store::in_memory().unwrap();
        index(
            &mut store,
            &host(),
            &[Root {
                agent,
                dir: home.path().to_owned(),
            }],
            &redactor(),
            NOW,
        )
        .unwrap();
        let session = &store.sessions(None, 1).unwrap()[0];
        let events = store.events(&session.id, 0, 10).unwrap();
        assert_eq!(events.len(), 1);
        let wire =
            String::from_utf8(crate::sync::encode(&store.sync_request("peer").unwrap()).unwrap())
                .unwrap();
        for leaked in [
            "hunter2",
            "tiny-key",
            "private-user",
            "private-value",
            "nested-secret",
        ] {
            assert!(
                !events[0].text.contains(leaked),
                "{agent:?}: {}",
                events[0].text
            );
            assert!(!wire.contains(leaked), "{agent:?} exported {leaked}");
        }
        assert!(events[0].text.contains("login"));
        assert!(events[0].text.contains("[redacted]"));
    }
}

#[test]
fn long_json_strings_are_scrubbed_before_truncation_breaks_their_syntax() {
    let arguments = json!({"command":"login", "env":{"PASSWORD":"hunter2"}, "padding":"x".repeat(MAX_TEXT * 2)}).to_string();
    let line = json!({"type":"response_item","payload":{"type":"function_call","name":"shell","arguments":arguments}}).to_string();
    let mut events = Vec::new();
    Parser::new().record(AgentKind::Codex, 0, &line, &redactor(), &mut events);
    assert!(!events[0].text.contains("hunter2"));
    assert!(events[0].text.contains("[redacted]"));
    assert!(events[0].text.len() <= MAX_TEXT);
}

#[test]
fn escaped_json_keys_and_recursively_encoded_json_strings_are_scrubbed() {
    let text = r#"{"ordinary":1,"nested":{"PASS\u0057ORD":"hunter2"}}"#;
    let encoded = serde_json::to_string(&serde_json::to_string(text).unwrap()).unwrap();
    let clean = redactor().redact(&encoded);
    assert!(!clean.contains("hunter2"));
    assert!(clean.contains("[redacted]"));
    let nested = (0..40).fold(json!("hunter2"), |value, _| json!({"child":value}));
    assert!(!redactor().redact(&nested.to_string()).contains("hunter2"));
}
