//! One bounded replication job per connected machine, across all app windows.
//! Host commands retain the existing SSH/relay/cloud/WSL authentication policy.
use super::service::{self, Mailbox, Snapshot, Status};
use crate::{SheprdWindow, teleport::Host, window::main_window::MainWindow};
use sheprd_client::ScriptLimits;
use sheprd_traces::{Store, sync};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const CONCURRENT: usize = 4;
const INTERVAL: Duration = Duration::from_secs(30);
const RETRY: Duration = Duration::from_secs(60);
const CATCH_UP: Duration = Duration::from_secs(1);
const ROUNDS: usize = 4;

pub(super) type Phase =
    sheprd_control_protocol::protocol::app::TraceSyncState<Arc<sheprd_traces::Error>>;
pub(super) type Report =
    sheprd_control_protocol::protocol::app::TracePeer<Arc<sheprd_traces::Error>>;

pub(super) struct Wanted {
    label: String,
    host: Host,
}

pub(super) fn wanted(cx: &gpui::App) -> BTreeMap<String, Wanted> {
    let mut wanted = BTreeMap::new();
    for window in MainWindow::all(cx) {
        if let Ok(view) = window.read(cx) {
            for (key, host) in hosts(view) {
                wanted.entry(key).or_insert(host);
            }
        }
    }
    wanted
}

fn hosts(view: &SheprdWindow) -> BTreeMap<String, Wanted> {
    view.endpoints
        .iter()
        .enumerate()
        .filter_map(|(index, endpoint)| {
            let live = if index == view.selected_endpoint {
                &view.live
            } else {
                &endpoint.live
            };
            let core = crate::core_terminals::Host::Remote(crate::browser::scope(endpoint));
            if !endpoint.enabled
                || !(live.status.is_connected() || view.core_terminals.is_connected(&core))
            {
                return None;
            }
            let host = Host::new(&endpoint.connection.target).ok()?;
            if host.is_local() {
                return None;
            }
            Some((
                host.machine_identity(),
                Wanted {
                    label: endpoint.label.clone(),
                    host,
                },
            ))
        })
        .collect()
}

struct Job {
    cancelled: Arc<AtomicBool>,
    result: mpsc::Receiver<sheprd_traces::Result<Outcome>>,
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

struct Outcome {
    more: bool,
    snapshot: Snapshot,
}

struct Peer {
    wanted: Wanted,
    next: Instant,
    phase: Phase,
    job: Option<Job>,
}

#[derive(Default)]
pub(super) struct Replication {
    peers: BTreeMap<String, Peer>,
}

impl Replication {
    pub(super) fn reports(&self) -> Vec<Report> {
        self.peers
            .values()
            .map(|peer| Report {
                host: peer.wanted.label.clone(),
                state: peer.phase.clone(),
            })
            .collect()
    }

    pub(super) fn poll(&mut self, wanted: BTreeMap<String, Wanted>, mailbox: &Mailbox) {
        let now = Instant::now();
        let previous = self.peers.len();
        self.peers.retain(|key, _| wanted.contains_key(key)); // Drop cancels detached hosts.
        if previous != self.peers.len() {
            mailbox.changed();
        }
        for (key, wanted) in wanted {
            if let Some(peer) = self.peers.get_mut(&key) {
                peer.wanted = wanted;
            } else {
                self.peers.insert(
                    key,
                    Peer {
                        wanted,
                        next: now,
                        phase: Phase::Waiting,
                        job: None,
                    },
                );
                mailbox.changed();
            }
        }
        for peer in self.peers.values_mut() {
            let Some(job) = &peer.job else { continue };
            let result = match job.result.try_recv() {
                Ok(result) => result,
                Err(mpsc::TryRecvError::Empty) => continue,
                Err(mpsc::TryRecvError::Disconnected) => Err(sync::Error::WorkerStopped.into()),
            };
            peer.job = None;
            match result {
                Ok(outcome) => {
                    peer.next = now + if outcome.more { CATCH_UP } else { INTERVAL };
                    peer.phase = if outcome.more {
                        Phase::Syncing
                    } else {
                        Phase::Ready
                    };
                    mailbox.put(Status::Ready(Arc::new(outcome.snapshot)));
                }
                Err(error) => {
                    peer.next = now + RETRY;
                    peer.phase = Phase::Failed {
                        error: Arc::new(error),
                    };
                    mailbox.changed();
                }
            }
        }
        let running = self
            .peers
            .values()
            .filter(|peer| peer.job.is_some())
            .count();
        let mut due: Vec<_> = self
            .peers
            .iter_mut()
            .filter(|(_, peer)| peer.job.is_none() && now >= peer.next)
            .collect();
        due.sort_by_key(|(_, peer)| peer.next);
        for (key, peer) in due.into_iter().take(CONCURRENT.saturating_sub(running)) {
            let cancelled = Arc::new(AtomicBool::new(false));
            let flag = cancelled.clone();
            let (sender, result) = mpsc::sync_channel(1);
            let host = peer.wanted.host.clone();
            let route = key.clone();
            match std::thread::Builder::new()
                .name("traces-replication".into())
                .spawn(move || {
                    let _ = sender.send(run(&host, &route, &flag));
                }) {
                Ok(_) => {
                    peer.job = Some(Job { cancelled, result });
                    peer.phase = Phase::Syncing;
                }
                Err(error) => {
                    peer.phase = Phase::Failed {
                        error: Arc::new(sync::Error::Io(error).into()),
                    };
                    peer.next = now + RETRY;
                }
            }
            mailbox.changed();
        }
    }
}

fn run(host: &Host, route: &str, cancelled: &AtomicBool) -> sheprd_traces::Result<Outcome> {
    let path = sheprd_traces::path().ok_or(sheprd_traces::Error::NoStateDirectory)?;
    let mut store = Store::open(&path)?;
    let mut more = true;
    for _ in 0..ROUNDS {
        if cancelled.load(Ordering::Acquire) {
            break;
        }
        more = sync::exchange(
            &mut store,
            route,
            chrono::Utc::now().timestamp_millis(),
            |input| {
                let mut output = Vec::new();
                host.stream(
                // i18n: data
                "if command -v sheprd-acp >/dev/null 2>&1; then exec sheprd-acp traces exchange; else exec herdr-acp traces exchange; fi\n",
                input,
                &mut output,
                ScriptLimits { output: sync::MAX_BYTES as u64, idle: Duration::from_secs(30) },
                cancelled,
            )?;
                if cancelled.load(Ordering::Acquire) {
                    return Err(sync::Error::Io(std::io::Error::from(
                        std::io::ErrorKind::Interrupted,
                    )));
                }
                Ok(output)
            },
        )?;
        if !more {
            break;
        }
    }
    Ok(Outcome {
        more,
        snapshot: service::snapshot(&store)?,
    })
}

#[cfg(test)]
mod tests;
