use crate::control::server::tabs::machine_claims::*;

fn caller(id: &str) -> Caller {
    Caller {
        terminal_id: Some(id.into()),
        ..Caller::default()
    }
}
fn busy(response: Response) {
    assert!(matches!(
        response,
        Response::Error {
            code: ErrorCode::Busy,
            ..
        }
    ));
}
fn done(response: Response) {
    assert!(matches!(response, Response::Done { .. }));
}

#[test]
fn reservations_exclude_other_panes_and_release_only_the_owner() {
    let mut reservations = Reservations::default();
    let now = Instant::now();
    done(reservations.change(&caller("a"), "one", false, now, Ok(())));
    busy(reservations.change(&caller("b"), "one", false, now, Ok(())));
    busy(reservations.change(&caller("b"), "one", true, now, Ok(())));
    assert!(matches!(
        reservations.check(&caller("b"), "one", now),
        Some(Response::Error {
            code: ErrorCode::Busy,
            ..
        })
    ));
    done(reservations.change(&caller("b"), "two", false, now, Ok(())));
    done(reservations.change(&caller("a"), "one", true, now, Ok(())));
    done(reservations.change(&caller("b"), "one", false, now, Ok(())));
}

#[test]
fn use_renews_and_expiry_allows_recovery_without_stale_release() {
    let mut reservations = Reservations::default();
    let now = Instant::now();
    done(reservations.change(&caller("a"), "one", false, now, Ok(())));
    assert!(
        reservations
            .check(&caller("a"), "one", now + IDLE / 2)
            .is_none()
    );
    busy(reservations.change(&caller("b"), "one", false, now + IDLE, Ok(())));
    done(reservations.change(&caller("b"), "one", false, now + IDLE * 2, Ok(())));
    busy(reservations.change(&caller("a"), "one", true, now + IDLE * 2, Ok(())));
}

#[test]
fn anonymous_callers_cannot_claim_and_daemons_disambiguate_panes() {
    let mut reservations = Reservations::default();
    let now = Instant::now();
    assert!(matches!(
        reservations.change(&Caller::default(), "one", false, now, Ok(())),
        Response::Error {
            code: ErrorCode::InvalidRequest,
            ..
        }
    ));
    let a = caller("same");
    let b = Caller {
        daemon_socket: Some("other".into()),
        ..a.clone()
    };
    done(reservations.change(&a, "one", false, now, Ok(())));
    busy(reservations.change(&b, "one", false, now, Ok(())));
}

#[test]
fn full_capacity_allows_renewal_and_release_but_not_new_or_expired_claims() {
    let mut reservations = Reservations::default();
    let now = Instant::now();
    let full = || {
        Err(crate::machine::Error::Capacity {
            in_use: 2,
            limit: 2,
        })
    };
    done(reservations.change(&caller("a"), "one", false, now, Ok(())));
    done(reservations.change(&caller("a"), "one", false, now + IDLE / 2, full()));
    assert_eq!(reservations.0["one"].until, now + IDLE / 2 + IDLE);
    busy(reservations.change(&caller("b"), "one", false, now + IDLE, full()));
    busy(reservations.change(&caller("b"), "two", false, now + IDLE, full()));
    done(reservations.change(&caller("a"), "one", true, now + IDLE, full()));
    busy(reservations.change(&caller("a"), "one", false, now + IDLE, full()));
    done(reservations.change(&caller("a"), "one", false, now + IDLE, Ok(())));
    busy(reservations.change(&caller("a"), "one", false, now + IDLE * 2, full()));
}
