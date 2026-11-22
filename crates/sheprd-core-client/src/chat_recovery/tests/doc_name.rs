use super::*;

/// herdr-core refuses a doc name that is not 1-32 of `a-z 0-9 - _ .`,
/// starting with a letter or digit; a refused name left chats unclosable.
#[test]
fn the_record_name_is_one_herdr_core_accepts() {
    for id in [ID, "ffffffffffffffffffffffffffffffff"] {
        let name = document(&id.parse().unwrap());
        assert!((1..=32).contains(&name.len()), "{name}");
        assert!(name.starts_with(|c: char| c.is_ascii_lowercase()), "{name}");
        assert!(
            name.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_.".contains(&b)),
            "{name}"
        );
    }
}

#[test]
fn closing_asks_for_the_record_by_its_short_name() {
    let (core, mut peer) = connected();
    let worker = std::thread::spawn(move || closed(&core, &ID.parse().unwrap()));
    let get = peer.request();
    assert_eq!(get["params"]["name"], "sheprd-chat-0123456789abcdef0123");
    peer.answer(&get, json!({"value": null, "version": 0}));
    let set = peer.request();
    assert_eq!(set["params"]["name"], "sheprd-chat-0123456789abcdef0123");
    peer.answer(&set, json!({"version": 1}));
    worker.join().unwrap().unwrap();
}
