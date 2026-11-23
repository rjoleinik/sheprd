use super::*;
use sheprd_remote_control::control::fleet::{Kind, MemberView};

fn member(id: &str) -> MemberView {
    MemberView {
        id: id.into(),
        name: id.into(),
        kind: Kind::Computer,
        admitted_by: None,
    }
}

fn machine(id: &str) -> (String, Machine) {
    (
        id.to_owned(),
        Machine {
            target: ConnectTarget::Ssh {
                target: id.to_owned(),
                session: "default".into(),
            },
            label: id.to_owned(),
        },
    )
}

#[test]
fn only_a_member_of_its_fleet_offers_it() {
    let mut view = fleet_view(&[]);
    view.members = vec![member("a"), member("b")];
    let offer = crate::fleet::join::Offer::of(&view).unwrap();
    assert_eq!(offer.fleet, view.fleet);
    assert_eq!((offer.members, offer.hosts), (2, 0));
    // A shared host holds the log but cannot admit anyone.
    view.me = "host".into();
    view.hosted = Some("host".into());
    assert_eq!(crate::fleet::join::Offer::of(&view), None);
}

#[test]
fn each_fleet_is_offered_once_with_every_member_reached() {
    let offer = |fleet: char| crate::fleet::join::Offer {
        fleet: fleet.to_string().repeat(64),
        members: 1,
        hosts: 0,
    };
    let mut fleet = Fleet::default();
    for (id, offered) in [
        ("laptop", Some(offer('a'))),
        ("studio", Some(offer('a'))),
        ("lab", Some(offer('b'))),
        ("box", None),
    ] {
        fleet.links.insert(
            id.to_owned(),
            Link {
                offer: offered,
                ..Link::default()
            },
        );
    }
    let reached: BTreeMap<_, _> = ["box", "lab", "laptop", "studio"].map(machine).into();
    let offered: Vec<_> = fleet
        .offered(reached.clone())
        .into_iter()
        .map(|offered| {
            let ids: Vec<_> = offered.through.iter().map(|(id, _)| id.clone()).collect();
            (offered.offer.fleet.chars().next(), ids)
        })
        .collect();
    assert_eq!(
        offered,
        vec![
            (Some('b'), vec!["lab".to_owned()]),
            (Some('a'), vec!["laptop".to_owned(), "studio".to_owned()]),
        ]
    );
    fleet.view = Some(fleet_view(&[]));
    assert!(fleet.offered(reached).is_empty());
}

#[test]
fn the_member_named_leads_and_the_others_keep_their_order() {
    let offered = crate::fleet::join::Offered {
        offer: crate::fleet::join::Offer {
            fleet: "a".repeat(64),
            members: 3,
            hosts: 0,
        },
        through: vec![machine("laptop"), machine("mini"), machine("studio")],
    };
    let ids = |through: Vec<(String, Machine)>| -> Vec<String> {
        through.into_iter().map(|(id, _)| id).collect()
    };
    assert_eq!(
        ids(offered.clone().through_first(Some("studio"))),
        ["studio", "laptop", "mini"]
    );
    assert_eq!(
        ids(offered.clone().through_first(None)),
        ["laptop", "mini", "studio"]
    );
    assert_eq!(
        ids(offered.through_first(Some("gone"))),
        ["laptop", "mini", "studio"]
    );
}

#[test]
fn a_machine_in_another_fleet_is_asked_rarely() {
    let (elsewhere, now) = link(Standing::Elsewhere, Duration::from_secs(600), &[]);
    assert!(!elsewhere.due(&[], now));
    let (elsewhere, now) = link(Standing::Elsewhere, OUTSIDE, &[]);
    assert!(elsewhere.due(&[], now));
}

#[test]
fn sheprd_in_no_fleet_stays_a_plain_host_of_this_one() {
    use crate::fleet::flows::beside_daemon;
    // Its relay agent belongs to this fleet: keep it current.
    assert_eq!(beside_daemon(Standing::Agent), Standing::Agent);
    assert_eq!(beside_daemon(Standing::Removed), Standing::Removed);
    // No agent of this fleet: a computer to invite.
    for plain in [Standing::NoDaemon, Standing::Outside, Standing::Unchecked] {
        assert_eq!(beside_daemon(plain), Standing::Outside);
    }
}
