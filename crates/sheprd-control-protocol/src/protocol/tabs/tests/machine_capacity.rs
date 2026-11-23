use crate::protocol::tabs::{MachineCapacity, Reply};

#[test]
fn capacity_counts_available_slots_without_underflow() {
    for (in_use, available) in [(0, 2), (1, 1), (2, 0), (3, 0)] {
        let capacity = MachineCapacity::new(in_use);
        assert_eq!(capacity.available, available);
        let reply = Reply::Machines {
            machines: Vec::new(),
            capacity: Some(capacity),
        };
        let json = serde_json::to_string(&reply).unwrap();
        assert_eq!(serde_json::from_str::<Reply>(&json).unwrap(), reply);
    }
}

#[test]
fn older_machine_lists_have_unknown_capacity() {
    let reply: Reply = serde_json::from_str(r#"{"kind":"machines","machines":[]}"#).unwrap();
    assert!(matches!(reply, Reply::Machines { capacity: None, .. }));
}
