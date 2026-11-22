use super::*;
use crate::{Tick, watch};
use std::{
    ops::ControlFlow,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

/// Generous for a loaded CI machine; the watcher itself settles in 50 ms.
const DEADLINE: Duration = Duration::from_secs(10);

/// Runs `watch` over `home` on a thread, sending each changed session's
/// message count.
fn start(home: &Path) -> (Arc<AtomicBool>, mpsc::Receiver<i64>, thread::JoinHandle<()>) {
    let stop = Arc::new(AtomicBool::new(false));
    let (sender, counts) = mpsc::channel();
    let home = home.to_owned();
    let flag = Arc::clone(&stop);
    let handle = thread::spawn(move || {
        let mut store = Store::in_memory().unwrap();
        watch(
            &mut store,
            &host(),
            &roots(&home),
            &redactor(),
            &flag,
            || NOW,
            |store, tick| {
                if let Tick::Changed(report) = tick {
                    for id in &report.sessions {
                        let session = store.session(id).unwrap().unwrap();
                        if sender.send(session.messages).is_err() {
                            return ControlFlow::Break(());
                        }
                    }
                }
                ControlFlow::Continue(())
            },
        )
        .unwrap();
    });
    (stop, counts, handle)
}

fn next(counts: &mpsc::Receiver<i64>, want: i64) {
    let until = Instant::now() + DEADLINE;
    while let Some(left) = until.checked_duration_since(Instant::now()) {
        match counts.recv_timeout(left) {
            Ok(count) if count == want => return,
            Ok(_) => {}
            Err(error) => panic!("no session with {want} messages: {error}"),
        }
    }
    panic!("no session with {want} messages in time");
}

#[test]
fn appended_messages_are_indexed_as_they_land() {
    let home = tempfile::tempdir().unwrap();
    let path = claude_file(home.path());
    write(&path, fixtures::CLAUDE);
    let (stop, counts, handle) = start(home.path());
    next(&counts, 3);
    append(&path, fixtures::CLAUDE_MORE);
    next(&counts, 4);
    stop.store(true, Ordering::Relaxed);
    handle.join().unwrap();
}

#[test]
fn an_agent_used_for_the_first_time_is_noticed() {
    let home = tempfile::tempdir().unwrap();
    // The existing agent's initial report confirms that all watches, including
    // the missing Pi directory's ancestor, are installed before Pi is created.
    write(&claude_file(home.path()), fixtures::CLAUDE);
    let (stop, counts, handle) = start(home.path());
    next(&counts, 3);
    // Neither ~/.pi nor anything under it exists when watching starts.
    write(
        &home
            .path()
            .join(".pi/agent/sessions/--home-me--/2026-11-17T09-00-00_7f3a-pi.jsonl"),
        fixtures::PI,
    );
    next(&counts, 2);
    stop.store(true, Ordering::Relaxed);
    handle.join().unwrap();
}

#[test]
fn watching_stops_when_its_reader_goes_away() {
    let home = tempfile::tempdir().unwrap();
    let path = claude_file(home.path());
    write(&path, fixtures::CLAUDE);
    let (_stop, counts, handle) = start(home.path());
    next(&counts, 3);
    drop(counts);
    append(&path, fixtures::CLAUDE_MORE);
    // The next change finds no reader and ends the watch on its own.
    handle.join().unwrap();
}
