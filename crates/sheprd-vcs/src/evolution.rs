//! Change history for a branch before it merges: every version of each of
//! its own commits, which agent's turn rewrote it, and how the rewrite was
//! recognized. The obsolescence markers of Mercurial's changeset evolution,
//! kept for one branch on its own host and forgotten once the branch is
//! merged or deleted; nothing is pushed.
//!
//! Nothing hooks into Git or changes a commit. Each time an agent starts or
//! finishes a turn, the checkpoint worker that snapshots the checkout also
//! compares the branch's commits with those it saw last, and records which
//! replaced which ([`matcher`]). Git keeps every replaced commit through the
//! history ref ([`script`]), so any two versions can still be diffed.

mod error;
pub mod log;
mod matcher;
mod script;
#[cfg(test)]
mod tests;

pub use error::Error;
pub use log::{Commit, Log, Marker, Match, Outcome};
pub use matcher::versions;

use crate::{Result, checkpoint::Checkout};
use sheprd_client::ScriptLimits;

/// Reads and writes that lose to another observer this many times give up;
/// the next turn tries again.
const ATTEMPTS: usize = 3;

/// Whose turn a look at the branch credits what changed since the last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Credit {
    /// A turn is starting: what changed was done outside an agent's turn.
    Outside,
    /// These agents just finished a turn.
    Agents(String),
}

impl Credit {
    /// One look standing for two that were queued for the same checkout.
    #[must_use]
    pub fn merge(self, newer: Self) -> Self {
        match (self, newer) {
            (Self::Agents(older), Self::Agents(newer)) => {
                if older.split(", ").any(|name| name == newer) {
                    Self::Agents(older)
                } else {
                    Self::Agents(log::clean(&format!("{older}, {newer}")))
                }
            }
            (Self::Agents(name), Self::Outside) | (Self::Outside, Self::Agents(name)) => {
                Self::Agents(name)
            }
            (Self::Outside, Self::Outside) => Self::Outside,
        }
    }

    fn by(&self) -> Option<String> {
        match self {
            Self::Outside => None,
            Self::Agents(names) => Some(log::clean(names)).filter(|names| !names.is_empty()),
        }
    }
}

/// What a look at the branch leads to.
#[derive(Debug, PartialEq, Eq)]
enum Plan {
    Nothing,
    /// Store `log`, moving the ref from `old`.
    Write {
        old: Option<String>,
        log: Log,
    },
    /// The branch has no commits of its own any more: it was merged.
    Delete {
        old: String,
    },
}

fn plan(read: script::Read, credit: &Credit, now: i64) -> Plan {
    let script::Read {
        head,
        mut log,
        stack,
    } = read;
    let Some(stack) = stack else {
        return Plan::Nothing;
    };
    if stack.is_empty() {
        return head.map_or(Plan::Nothing, |old| Plan::Delete { old });
    }
    let changes = matcher::evolve(&log.stack, &stack);
    if changes.is_empty() && log.stack == stack && head.is_some() {
        return Plan::Nothing;
    }
    let by = credit.by();
    log.record(changes.into_iter().map(|(old, outcome)| Marker {
        time: now,
        old,
        outcome,
        by: by.clone(),
    }));
    log.stack = stack;
    Plan::Write { old: head, log }
}

/// Look at `checkout`'s branch and record what replaced what since the last
/// look, crediting `credit`. `run` runs a script on the checkout's host with
/// the given stdin, as [`crate::checkpoint::direct::here_with_input`] does.
/// Blocking: run on a checkpoint worker or the command line.
pub fn observe(
    checkout: &Checkout,
    credit: &Credit,
    now: i64,
    mut run: impl FnMut(&Checkout, &str, &[u8], ScriptLimits) -> Result<String>,
) -> Result<()> {
    let limits = crate::checkpoint::LIMITS;
    for _ in 0..ATTEMPTS {
        let read = script::parse_read(&run(checkout, &script::read(checkout), b"", limits)?)?;
        let output = match plan(read, credit, now) {
            Plan::Nothing => return Ok(()),
            Plan::Write { old, log } => {
                let body = script::write(checkout, old.as_deref(), &log.kept());
                run(checkout, &body, log.render().as_bytes(), limits)?
            }
            Plan::Delete { old } => run(checkout, &script::delete(checkout, &old), b"", limits)?,
        };
        if script::parse_write(&output)? {
            return Ok(());
        }
    }
    Err(Error::Raced.into())
}

/// A branch's history as it stands now, for reading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct History {
    /// The branch's own commits, oldest first.
    pub stack: Vec<Commit>,
    /// Recorded markers, then those since the last look.
    pub markers: Vec<Marker>,
    /// How many of the last `markers` happened since the last look, so are
    /// not credited to anyone yet.
    pub pending: usize,
}

impl History {
    /// Whether `marker`, one of [`History::markers`], is not recorded yet.
    pub fn is_pending(&self, marker: &Marker) -> bool {
        let start = self.markers.len().saturating_sub(self.pending);
        self.markers[start..]
            .iter()
            .any(|pending| std::ptr::eq(pending, marker))
    }
}

/// `checkout`'s branch history, with what changed since the last look but
/// without recording it. `None` when the branch has no commits of its own
/// to track: it is the default branch, or has no base. Reads only; `now`
/// stamps the markers not recorded yet.
pub fn history(
    checkout: &Checkout,
    now: i64,
    mut run: impl FnMut(&Checkout, &str, &[u8], ScriptLimits) -> Result<String>,
) -> Result<Option<History>> {
    let limits = crate::checkpoint::LIMITS;
    let read = script::parse_read(&run(checkout, &script::read(checkout), b"", limits)?)?;
    let Some(stack) = read.stack else {
        return Ok(None);
    };
    let mut markers = read.log.markers;
    let pending: Vec<Marker> = matcher::evolve(&read.log.stack, &stack)
        .into_iter()
        .map(|(old, outcome)| Marker {
            time: now,
            old,
            outcome,
            by: None,
        })
        .collect();
    let count = pending.len();
    markers.extend(pending);
    Ok(Some(History {
        stack,
        markers,
        pending: count,
    }))
}
