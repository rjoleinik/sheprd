use super::*;

#[test]
fn traces_status_is_typed_and_refuses_extra_request_fields() {
    assert_eq!(
        serde_json::from_str::<UiOp>(r#"{"op":"traces"}"#).unwrap(),
        UiOp::Traces {}
    );
    assert!(serde_json::from_str::<UiOp>(r#"{"op":"traces","connect":true}"#).is_err());
    let reply = Reply::Traces {
        filter: TraceFilter::All,
        hosts: Vec::new(),
        peers: vec![TracePeer {
            host: "box".into(),
            state: TraceSyncState::Failed {
                error: "helper missing".into(),
            },
        }],
    };
    let encoded = serde_json::to_string(&reply).unwrap();
    assert_eq!(serde_json::from_str::<Reply>(&encoded).unwrap(), reply);
}
