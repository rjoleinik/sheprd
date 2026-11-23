use crate::control::{
    client::tabs::machines,
    protocol::{
        Response,
        tabs::{MachineCapacity, Reply},
    },
};

#[test]
fn full_capacity_is_reported_to_people_and_agents() {
    let output = machines(Response::Tab {
        reply: Reply::Machines {
            machines: Vec::new(),
            capacity: Some(MachineCapacity::new(2)),
        },
    })
    .unwrap();
    assert!(output.text.contains("2 of 2 in use, 0 available"));
    assert!(output.text.contains("No VM slots available"));
    assert_eq!(output.json["capacity"]["in_use"], 2);
    assert_eq!(output.json["capacity"]["limit"], 2);
    assert_eq!(output.json["capacity"]["available"], 0);
}
