//! `fleet invite` and `fleet join` name machines this Sheprd reaches,
//! not members: they resolve against what Settings > Fleet would offer.
use super::*;

fn machines(names: &[&str]) -> Vec<(String, String)> {
    names
        .iter()
        .map(|name| (format!("id-{name}"), (*name).to_owned()))
        .collect()
}

#[test]
fn invite_asks_for_one_machine_or_all_of_them() {
    let outside = machines(&["m4max", "m4-pro-1.local"]);
    assert_eq!(
        invite_pending(true, &outside, Some("m4max")).unwrap(),
        FleetPending::Invite("id-m4max".into())
    );
    assert_eq!(
        invite_pending(true, &outside, None).unwrap(),
        FleetPending::InviteAll
    );
    assert!(matches!(
        invite_pending(true, &outside, Some("studio")),
        Err(Response::Error {
            code: ErrorCode::NotFound,
            ..
        })
    ));
    assert!(matches!(
        invite_pending(true, &[], None),
        Err(Response::Error {
            code: ErrorCode::NotFound,
            ..
        })
    ));
    // Only a member invites.
    assert!(invite_pending(false, &outside, None).is_err());
}

#[test]
fn join_needs_this_computer_in_no_fleet_and_a_fleet_to_join() {
    let offers = machines(&["studio"]);
    assert_eq!(
        join_pending(false, &offers, None).unwrap(),
        FleetPending::Join(None)
    );
    // The member named is the one joined through.
    assert_eq!(
        join_pending(false, &offers, Some("studio")).unwrap(),
        FleetPending::Join(Some("id-studio".into()))
    );
    assert!(join_pending(false, &offers, Some("lab")).is_err());
    assert!(join_pending(false, &[], None).is_err());
    assert!(matches!(
        join_pending(true, &offers, None),
        Err(Response::Error {
            code: ErrorCode::InvalidRequest,
            ..
        })
    ));
}
