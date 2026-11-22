//! Mirror device-tag edits into the index on a worker. The replica handshake
//! supplies route -> stable host ID; host names are never used as identities.
use super::super::service::{self, Mailbox, Snapshot, Status};
use crate::{device_tags::DeviceTags, teleport::Host, window::main_window::MainWindow};
use sheprd_client::device_tags::ComputerTags;
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

type Wanted = BTreeMap<String, ComputerTags>;

pub(in crate::traces) fn wanted(cx: &gpui::App, status: &Status) -> Wanted {
    let Status::Ready(snapshot) = status else {
        return Wanted::new();
    };
    let tags = DeviceTags::current(cx);
    let mut wanted = Wanted::new();
    for window in MainWindow::all(cx) {
        let Ok(view) = window.read(cx) else { continue };
        for endpoint in &view.endpoints {
            let Some(record) = tags.record(&endpoint.id) else {
                continue;
            };
            let Ok(host) = Host::new(&endpoint.connection.target) else {
                continue;
            };
            let id = if host.is_local() {
                snapshot.local_host.as_ref()
            } else {
                snapshot.routes.get(&host.machine_identity())
            };
            let Some(id) = id else { continue };
            let current = wanted.entry(id.clone()).or_default();
            if record.updated_ms > current.updated_ms {
                *current = record.clone();
            }
        }
    }
    wanted
}

struct Job {
    wanted: Wanted,
    cancelled: Arc<AtomicBool>,
    result: mpsc::Receiver<sheprd_traces::Result<Snapshot>>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

#[derive(Default)]
pub(in crate::traces) struct Publisher {
    applied: Wanted,
    job: Option<Job>,
    retry: Option<Instant>,
}

impl Publisher {
    pub(in crate::traces) fn poll(&mut self, wanted: Wanted, mailbox: &Mailbox) {
        if let Some(job) = &self.job {
            match job.result.try_recv() {
                Ok(Ok(snapshot)) => {
                    self.applied = job.wanted.clone();
                    mailbox.put(Status::Ready(Arc::new(snapshot)));
                    self.job = None;
                }
                Ok(Err(error)) => {
                    tracing::warn!(%error, "could not mirror device tags into traces");
                    self.job = None;
                    self.retry = Some(Instant::now() + Duration::from_secs(30));
                }
                Err(mpsc::TryRecvError::Empty) => return,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.job = None;
                    self.retry = Some(Instant::now() + Duration::from_secs(30));
                }
            }
        }
        if self.applied == wanted || self.retry.is_some_and(|at| Instant::now() < at) {
            return;
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = cancelled.clone();
        let copy = wanted.clone();
        let (sender, result) = mpsc::sync_channel(1);
        match std::thread::Builder::new()
            .name("traces-tags".into())
            .spawn(move || {
                let _ = sender.send(write(copy, &flag));
            }) {
            Ok(_) => {
                self.job = Some(Job {
                    wanted,
                    cancelled,
                    result,
                })
            }
            Err(error) => {
                tracing::warn!(%error, "could not start trace tag writer");
                self.retry = Some(Instant::now() + Duration::from_secs(30));
            }
        }
    }
}

fn write(wanted: Wanted, cancelled: &AtomicBool) -> sheprd_traces::Result<Snapshot> {
    let path = sheprd_traces::path().ok_or(sheprd_traces::Error::NoStateDirectory)?;
    let mut store = sheprd_traces::Store::open(&path)?;
    let now = chrono::Utc::now().timestamp_millis().max(0) as u64;
    for (host, tags) in wanted {
        if cancelled.load(Ordering::Acquire) {
            break;
        }
        store.set_host_tags(&host, tags, now)?;
    }
    service::snapshot(&store)
}
