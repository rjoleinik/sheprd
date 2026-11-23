use crate::{
    control::protocol::tabs::MachineCapacity,
    machine::{
        Error,
        vm::{Status, check_capacity_for},
    },
};

#[test]
fn full_capacity_refuses_new_starts_but_allows_existing_guests() {
    let full = MachineCapacity::new(2);
    for status in [Status::Stopped, Status::Failed("previous failure".into())] {
        assert!(matches!(
            check_capacity_for(status, full),
            Err(Error::Capacity {
                in_use: 2,
                limit: 2
            })
        ));
    }
    for status in [Status::Starting, Status::Running, Status::Saving] {
        assert!(check_capacity_for(status, full).is_ok());
    }
    assert!(check_capacity_for(Status::Stopped, MachineCapacity::new(1)).is_ok());
}
