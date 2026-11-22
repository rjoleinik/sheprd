use super::*;
use sheprd_client::device_tags::ComputerTags;

#[test]
fn device_tags_and_clears_reach_indirect_hosts_without_echo_growth() {
    let (mut a, mut b, mut c) = (
        Store::in_memory().unwrap(),
        Store::in_memory().unwrap(),
        Store::in_memory().unwrap(),
    );
    let (_dir, host, _) = source(&mut a, "A", 2);
    let work = ComputerTags::new(["Work"], NOW as u64);
    assert!(a.set_host_tags(&host.id, work.clone(), NOW as u64).unwrap());
    converge(&mut a, &mut b, "B");
    converge(&mut b, &mut c, "C");
    assert_eq!(c.hosts().unwrap()[0].tags, work);
    assert_eq!(b.sync_peers().unwrap()[0].host_id, None); // C has no local source.
    converge(&mut b, &mut a, "A");
    assert_eq!(
        b.sync_peers()
            .unwrap()
            .iter()
            .find(|peer| peer.route == "A")
            .unwrap()
            .host_id
            .as_deref(),
        Some(host.id.as_str())
    );
    let position = |store: &Store| -> i64 {
        store
            .connection
            .query_row("SELECT max(position) FROM trace_host_tags", [], |row| {
                row.get(0)
            })
            .unwrap()
    };
    let before = position(&a);
    converge(&mut c, &mut a, "A");
    assert_eq!(position(&a), before);
    let cleared = ComputerTags::new(Vec::<String>::new(), NOW as u64 + 1);
    a.set_host_tags(&host.id, cleared.clone(), NOW as u64 + 1)
        .unwrap();
    converge(&mut a, &mut b, "B");
    converge(&mut b, &mut c, "C");
    converge(&mut c, &mut a, "A");
    for store in [&a, &b, &c] {
        assert_eq!(store.hosts().unwrap()[0].tags, cleared);
    }
}

#[test]
fn tag_deltas_paginate_even_when_replacing_rows_leaves_sequence_gaps() {
    let (mut a, mut b) = (Store::in_memory().unwrap(), Store::in_memory().unwrap());
    for i in 0..MAX_RECORDS + 3 {
        let id = Host::id_of(&format!("host-{i}"));
        a.set_host_tags(&id, ComputerTags::new(["work"], 1), 10)
            .unwrap();
        a.set_host_tags(&id, ComputerTags::new(["personal"], 2), 10)
            .unwrap();
    }
    converge(&mut a, &mut b, "B");
    let count: i64 = b
        .connection
        .query_row("SELECT count(*) FROM trace_host_tags", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, (MAX_RECORDS + 3) as i64);
    assert_eq!(
        a.sync_peers().unwrap()[0].tags_sent,
        ((MAX_RECORDS + 3) * 2) as i64
    );
}

#[test]
fn a_malformed_tag_batch_rolls_back_session_import_too() {
    let (mut a, mut b) = (Store::in_memory().unwrap(), Store::in_memory().unwrap());
    let (_dir, host, _) = source(&mut a, "A", 1);
    a.set_host_tags(&host.id, ComputerTags::new(["work"], 1), 10)
        .unwrap();
    step(&mut a, &mut b, "B");
    let mut value = serde_json::to_value(a.sync_request("B").unwrap()).unwrap();
    value["tags_push"]["entries"][0]["tags"]["tags"] = serde_json::json!(["work", "WORK"]);
    assert!(matches!(
        b.sync_respond(serde_json::from_value(value).unwrap()),
        Err(crate::Error::Sync(Error::Invalid("tag entry")))
    ));
    assert_eq!(log_len(&b), 0);
    assert!(b.hosts().unwrap().is_empty());
}

#[test]
fn activity_filters_source_hosts_in_sql_without_dropping_daily_totals() {
    let (mut a, mut b) = (Store::in_memory().unwrap(), Store::in_memory().unwrap());
    let (_a, host_a, _) = source(&mut a, "A", 2);
    let (_b, host_b, _) = source(&mut b, "B", 3);
    converge(&mut a, &mut b, "B");
    let messages = |hosts: Option<&[&str]>| {
        a.activity_for(NOW - 1, NOW + 1, hosts)
            .unwrap()
            .iter()
            .map(|day| day.messages)
            .sum::<i64>()
    };
    assert_eq!(messages(None), 5);
    assert_eq!(messages(Some(&[&host_a.id])), 2);
    assert_eq!(messages(Some(&[&host_b.id])), 3);
    assert_eq!(messages(Some(&[&host_a.id, &host_b.id])), 5);
    assert_eq!(messages(Some(&[])), 0);
}
