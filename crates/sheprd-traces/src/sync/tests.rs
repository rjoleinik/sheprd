#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use crate::{Host, Root, index, redact::Redactor};
use sheprd_client::herdr::AgentKind;
use std::{fs, io::Write};

const NOW: i64 = 1_794_916_800_000;

mod tags;

fn source(store: &mut Store, name: &str, count: usize) -> (tempfile::TempDir, Host, Root) {
    let dir = tempfile::tempdir().unwrap();
    let host = Host {
        id: Host::id_of(name),
        label: name.into(),
        os: "linux".into(),
        user: format!("me@{name}"),
    };
    let root = Root {
        agent: AgentKind::Claude,
        dir: dir.path().to_owned(),
    };
    let mut file = fs::File::create(dir.path().join("session.jsonl")).unwrap();
    for i in 0..count {
        writeln!(file, "{}", serde_json::json!({"type":"user", "sessionId":"session", "timestamp":"2026-11-17T12:00:00Z", "message":{"role":"user","content":format!("{name} message {i}")}})).unwrap();
    }
    index(
        store,
        &host,
        std::slice::from_ref(&root),
        &Redactor::new(None),
        NOW,
    )
    .unwrap();
    (dir, host, root)
}

fn step(a: &mut Store, b: &mut Store, route: &str) -> bool {
    let request = a.sync_request(route).unwrap();
    let response = b
        .sync_respond(decode(&encode(&request).unwrap()).unwrap())
        .unwrap();
    a.sync_complete(
        route,
        &request,
        decode(&encode(&response).unwrap()).unwrap(),
        NOW,
    )
    .unwrap()
}

fn converge(a: &mut Store, b: &mut Store, route: &str) {
    for _ in 0..100 {
        if !step(a, b, route) {
            return;
        }
    }
    panic!("replication did not converge");
}

fn log_len(store: &Store) -> i64 {
    store
        .connection
        .query_row("SELECT count(*) FROM trace_journal", [], |row| row.get(0))
        .unwrap()
}

#[test]
fn three_hosts_forward_every_origin_without_echo_growth() {
    let (mut a, mut b, mut c) = (
        Store::in_memory().unwrap(),
        Store::in_memory().unwrap(),
        Store::in_memory().unwrap(),
    );
    let _a = source(&mut a, "A", 3);
    let _b = source(&mut b, "B", 2);
    let _c = source(&mut c, "C", 1);
    converge(&mut a, &mut b, "B");
    converge(&mut a, &mut c, "C");
    converge(&mut a, &mut b, "B");
    for store in [&a, &b, &c] {
        assert_eq!(store.sessions(None, 100).unwrap().len(), 3);
        assert_eq!(
            store
                .activity(0)
                .unwrap()
                .iter()
                .map(|day| day.messages)
                .sum::<i64>(),
            6
        );
    }
    let lengths = [log_len(&a), log_len(&b), log_len(&c)];
    converge(&mut c, &mut b, "B");
    converge(&mut b, &mut a, "A");
    converge(&mut c, &mut a, "A");
    assert_eq!([log_len(&a), log_len(&b), log_len(&c)], lengths);
}

#[test]
fn lost_reply_replays_without_duplicate_events_or_totals() {
    let (mut a, mut b) = (Store::in_memory().unwrap(), Store::in_memory().unwrap());
    let _a = source(&mut a, "A", 3);
    assert!(step(&mut a, &mut b, "B")); // identity handshake
    let request = a.sync_request("B").unwrap();
    b.sync_respond(request).unwrap(); // simulate a lost reply
    converge(&mut a, &mut b, "B");
    assert_eq!(b.sessions(None, 1).unwrap()[0].messages, 3);
    assert_eq!(log_len(&b), log_len(&a));
    assert_eq!(a.sync_peers().unwrap()[0].synced_at, Some(NOW));
}

#[test]
fn bounded_batches_catch_up_without_truncating_history() {
    let (mut a, mut b) = (Store::in_memory().unwrap(), Store::in_memory().unwrap());
    let _a = source(&mut a, "A", MAX_RECORDS * MAX_EVENTS + 7);
    let request = a.sync_request("B").unwrap();
    assert!(encode(&request).unwrap().len() < MAX_BYTES);
    converge(&mut a, &mut b, "B");
    assert_eq!(
        b.sessions(None, 1).unwrap()[0].messages,
        (MAX_RECORDS * MAX_EVENTS + 7) as i64
    );
    assert_eq!(log_len(&a), log_len(&b));
}

#[test]
fn reset_reaches_replicas_and_stale_peers_cannot_resurrect_events() {
    let (mut a, mut b, mut c) = (
        Store::in_memory().unwrap(),
        Store::in_memory().unwrap(),
        Store::in_memory().unwrap(),
    );
    let (dir, host, root) = source(&mut a, "A", 3);
    converge(&mut a, &mut b, "B");
    converge(&mut b, &mut c, "C");
    fs::write(
        dir.path().join("session.jsonl"),
        "{\"type\":\"user\",\"message\":{\"content\":\"replacement\"}}\n",
    )
    .unwrap();
    index(&mut a, &host, &[root], &Redactor::new(None), NOW).unwrap();
    converge(&mut a, &mut c, "C");
    converge(&mut c, &mut b, "B");
    for store in [&a, &b, &c] {
        let session = &store.sessions(None, 1).unwrap()[0];
        assert_eq!(session.messages, 1);
        assert_eq!(
            store.events(&session.id, 0, 10).unwrap()[0].text,
            "replacement"
        );
    }
}

#[test]
fn replacing_a_peers_database_restarts_its_cursors() {
    let (mut a, mut b) = (Store::in_memory().unwrap(), Store::in_memory().unwrap());
    let _a = source(&mut a, "A", 2);
    converge(&mut a, &mut b, "B");
    let old = a.sync_peers().unwrap()[0].replica.clone();
    b = Store::in_memory().unwrap();
    converge(&mut a, &mut b, "B");
    assert_ne!(a.sync_peers().unwrap()[0].replica, old);
    assert_eq!(b.sessions(None, 1).unwrap()[0].messages, 2);
}

#[test]
fn a_rebuilt_source_preserves_its_current_files_over_old_replicas() {
    let (mut a, mut b) = (Store::in_memory().unwrap(), Store::in_memory().unwrap());
    let (dir, host, root) = source(&mut a, "A", 3);
    // Give the old generation a clock strictly newer than a fresh database.
    fs::write(
        dir.path().join("session.jsonl"),
        "{\"type\":\"user\",\"message\":{\"content\":\"old\"}}\n",
    )
    .unwrap();
    index(
        &mut a,
        &host,
        std::slice::from_ref(&root),
        &Redactor::new(None),
        NOW,
    )
    .unwrap();
    converge(&mut a, &mut b, "B");
    a = Store::in_memory().unwrap();
    fs::write(
        dir.path().join("session.jsonl"),
        "{\"type\":\"user\",\"message\":{\"content\":\"current\"}}\n",
    )
    .unwrap();
    index(&mut a, &host, &[root], &Redactor::new(None), NOW).unwrap();
    converge(&mut a, &mut b, "B");
    // A rebase created while applying a reply is sent on the following pass.
    converge(&mut a, &mut b, "B");
    for store in [&a, &b] {
        let session = &store.sessions(None, 1).unwrap()[0];
        assert_eq!(store.events(&session.id, 0, 10).unwrap()[0].text, "current");
        assert_eq!(session.messages, 1);
    }
}

#[test]
fn malformed_batch_rolls_back_every_record_and_checkpoint() {
    let (mut a, mut b) = (Store::in_memory().unwrap(), Store::in_memory().unwrap());
    let _a = source(&mut a, "A", MAX_EVENTS + 1);
    step(&mut a, &mut b, "B");
    let request = a.sync_request("B").unwrap();
    let mut value = serde_json::to_value(request).unwrap();
    value["push"]["records"][1]["sequence"] = 999.into();
    let error = b
        .sync_respond(serde_json::from_value(value).unwrap())
        .unwrap_err();
    assert!(matches!(error, crate::Error::Sync(Error::Gap)));
    assert!(b.sessions(None, 10).unwrap().is_empty());
    assert_eq!(log_len(&b), 0);
}

#[test]
fn protocol_bounds_and_reply_correlation_are_enforced() {
    assert!(matches!(
        decode::<Request>(&vec![b' '; MAX_BYTES + 1]),
        Err(Error::Limit)
    ));
    let (mut a, mut b) = (Store::in_memory().unwrap(), Store::in_memory().unwrap());
    let request = a.sync_request("B").unwrap();
    let other = a.sync_request("B").unwrap();
    let reply = b.sync_respond(other).unwrap();
    assert!(matches!(
        a.sync_complete("B", &request, reply, NOW),
        Err(crate::Error::Sync(Error::Correlation))
    ));
    assert!(a.sync_peers().unwrap().is_empty());
    let mut value = serde_json::to_value(request).unwrap();
    value["protocol"] = (PROTOCOL + 1).into();
    assert!(matches!(
        b.sync_respond(serde_json::from_value(value).unwrap()),
        Err(crate::Error::Sync(Error::Protocol(_)))
    ));
}

#[test]
fn a_peer_cannot_exhaust_the_clock_with_an_unbacked_jump() {
    let (mut a, mut b) = (Store::in_memory().unwrap(), Store::in_memory().unwrap());
    let _a = source(&mut a, "A", 1);
    step(&mut a, &mut b, "B");
    let mut value = serde_json::to_value(a.sync_request("B").unwrap()).unwrap();
    value["push"]["records"][0]["version"]["clock"] = i64::MAX.into();
    assert!(matches!(
        b.sync_respond(serde_json::from_value(value).unwrap()),
        Err(crate::Error::Sync(Error::Gap))
    ));
    assert_eq!(log_len(&b), 0);
    let _b = source(&mut b, "B", 1);
    converge(&mut a, &mut b, "B");
    assert_eq!(b.sessions(None, 10).unwrap().len(), 2);
}

#[test]
fn schema_one_history_is_seeded_once_for_replication() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private/traces.db");
    let mut a = Store::open(&path).unwrap();
    let _a = source(&mut a, "A", MAX_EVENTS + 1);
    a.connection.execute_batch("DROP TABLE trace_host_tags; DROP TABLE trace_peers; DROP TABLE trace_versions; DROP TABLE trace_journal; DROP TABLE trace_heads; DROP TABLE trace_replica; PRAGMA user_version=1;").unwrap();
    drop(a);
    let mut a = Store::open(&path).unwrap();
    let mut b = Store::in_memory().unwrap();
    converge(&mut a, &mut b, "B");
    assert_eq!(
        b.sessions(None, 1).unwrap()[0].messages,
        (MAX_EVENTS + 1) as i64
    );
    let length = log_len(&a);
    drop(a);
    assert_eq!(log_len(&Store::open(&path).unwrap()), length);
}

#[test]
fn renaming_a_host_does_not_change_the_sessions_account_identity() {
    let (mut a, mut b) = (Store::in_memory().unwrap(), Store::in_memory().unwrap());
    let (dir, mut host, root) = source(&mut a, "A", 1);
    converge(&mut a, &mut b, "B");
    host.label = "renamed".into();
    host.user = "me@renamed".into();
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(dir.path().join("session.jsonl"))
        .unwrap();
    writeln!(
        file,
        "{}",
        serde_json::json!({"type":"user","message":{"content":"after rename"}})
    )
    .unwrap();
    index(&mut a, &host, &[root], &Redactor::new(None), NOW).unwrap();
    converge(&mut a, &mut b, "B");
    let session = &b.sessions(None, 1).unwrap()[0];
    assert_eq!(session.host, "renamed");
    assert_eq!(session.messages, 2);
}

#[test]
fn stdio_exchange_is_bounded_and_a_transport_error_keeps_progress() {
    let (mut a, mut b) = (Store::in_memory().unwrap(), Store::in_memory().unwrap());
    let _a = source(&mut a, "A", 1);
    for _ in 0..10 {
        let more = exchange(&mut a, "B", NOW, |input| {
            let mut output = Vec::new();
            serve(&mut b, input, &mut output).unwrap();
            Ok(output)
        })
        .unwrap();
        if !more {
            break;
        }
    }
    let before = a.sync_peers().unwrap()[0].sent;
    let error = exchange(&mut a, "B", NOW, |_| {
        Err(Error::Io(std::io::ErrorKind::BrokenPipe.into()))
    })
    .unwrap_err();
    assert!(matches!(error, crate::Error::Sync(Error::Io(_))));
    assert!(std::error::Error::source(&error).is_some());
    assert_eq!(a.sync_peers().unwrap()[0].sent, before);
    let mut output = Vec::new();
    assert!(matches!(
        serve(&mut b, &vec![b' '; MAX_BYTES + 1][..], &mut output),
        Err(crate::Error::Sync(Error::Limit))
    ));
    assert!(output.is_empty());
}
