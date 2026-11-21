//! `desktop-pair`: adds a nearby Sheprd, with Sheprd carrying Bluetooth
//! over stdio (`crate::control::desktop`). The code exchange's frames go
//! out and come back as lines; the claim goes through the other computer's
//! relays; the machine is saved once its user allows it.

use super::{
    Error, Keys, Result,
    claim::{Claimant, claim},
    exchange::Exchange,
    keys::ssh_public_key,
};
use crate::control::{
    desktop::{MAX_LINE, PairInput, PairOutput, frame_bytes},
    fleet::bluetooth::{ClaimFleet, pairing_digest},
};
use sheprd_client::relay_host::host_name;
use std::{
    io::{self, BufRead as _, Read as _, Write as _},
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::sync::mpsc;

/// How long the other computer has to answer each Bluetooth step.
const STEP: Duration = Duration::from_secs(30);
/// How long the person here has to say what the claim asks of the fleet:
/// within the minute the other computer's sealed token lasts.
const DECIDE: Duration = Duration::from_secs(50);

/// What Sheprd hands the child.
enum Input {
    /// What the other computer answered over Bluetooth.
    Frame(Vec<u8>),
    /// What the claim asks of the fleet.
    Claim(Option<ClaimFleet>),
}

/// Writes one line to Sheprd.
pub(crate) fn emit(output: &PairOutput) {
    let Ok(mut line) = serde_json::to_vec(output) else {
        return;
    };
    line.push(b'\n');
    let mut stdout = io::stdout().lock();
    let _ = stdout.write_all(&line).and_then(|()| stdout.flush());
}

/// Runs the pairing, reporting each step on stdout; the error is what the
/// person reads.
pub(crate) async fn pair(state_dir: PathBuf, name: String) -> Result<()> {
    let name = crate::auth::device_name(&name)?;
    let mut frames = frames();
    let state = state_dir.clone();
    let (keys, ssh_key) = tokio::task::spawn_blocking(move || {
        Ok::<_, Error>((Keys::load_or_create(&state)?, ssh_public_key(&state)?))
    })
    .await
    .map_err(|_| Error::Cancelled)??;

    let (mut exchange, start) = Exchange::start()?;
    emit(&PairOutput::frame(&start));
    let reveal = exchange.reveal(&next(&mut frames).await?)?;
    emit(&PairOutput::frame(&reveal));
    let opened = exchange.open(&next(&mut frames).await?)?;
    emit(&PairOutput::Code {
        code: opened.code.clone(),
        computer: opened.computer.clone(),
        fleet: opened.fleet.clone(),
        pairing: crate::control::fleet::sync::text(&pairing_digest(&opened.token)),
    });
    let fleet = decision(&mut frames).await?;

    let claimant = Claimant {
        keys: &keys,
        name: &name,
        ssh_key: &ssh_key,
        fleet: fleet.as_ref(),
    };
    // Sheprd closing this child's input cancels, whatever step it is on.
    let allowed = tokio::select! {
        allowed = claim(&opened, &claimant, || emit(&PairOutput::Claimed)) => allowed?,
        () = closed(&mut frames) => return Err(Error::Cancelled),
    };
    let state = state_dir.clone();
    let record = allowed.record.clone();
    let host_keys = allowed.login.host_keys.clone();
    let id = tokio::task::spawn_blocking(move || record.save(&state, &host_keys))
        .await
        .map_err(|_| Error::Cancelled)??;
    emit(&PairOutput::Paired {
        target: format!("{}@{}", allowed.login.user, host_name(&id)),
        label: allowed.record.computer,
        fleet: allowed.fleet,
    });
    Ok(())
}

/// Frames Sheprd hands back, read on a thread of their own; the channel
/// closes with Sheprd's end of the pipe.
fn frames() -> mpsc::Receiver<Input> {
    let (send, receive) = mpsc::channel(16);
    std::thread::spawn(move || {
        let mut stdin = io::stdin().lock();
        let mut line = String::new();
        loop {
            line.clear();
            match (&mut stdin).take(MAX_LINE as u64 + 1).read_line(&mut line) {
                Ok(0) | Err(_) => return,
                Ok(_) if line.len() > MAX_LINE => return,
                Ok(_) => {}
            }
            let input = match serde_json::from_str(line.trim_end()) {
                Ok(PairInput::Frame { data }) => match frame_bytes(&data) {
                    Some(frame) => Input::Frame(frame),
                    None => continue,
                },
                Ok(PairInput::Claim { fleet }) => Input::Claim(fleet),
                Err(_) => continue,
            };
            if send.blocking_send(input).is_err() {
                return;
            }
        }
    });
    receive
}

/// The next frame, dropping a decision out of turn.
async fn next(frames: &mut mpsc::Receiver<Input>) -> Result<Vec<u8>> {
    tokio::time::timeout(STEP, async {
        loop {
            match frames.recv().await {
                Some(Input::Frame(frame)) => return Ok(frame),
                Some(Input::Claim(_)) => {}
                None => return Err(Error::Cancelled),
            }
        }
    })
    .await
    .map_err(|_| Error::Expired)?
}

/// What the person here decided the claim asks of the fleet, dropping any
/// late frame.
async fn decision(frames: &mut mpsc::Receiver<Input>) -> Result<Option<ClaimFleet>> {
    tokio::time::timeout(DECIDE, async {
        loop {
            match frames.recv().await {
                Some(Input::Claim(fleet)) => return Ok(fleet),
                Some(Input::Frame(_)) => {}
                None => return Err(Error::Cancelled),
            }
        }
    })
    .await
    .map_err(|_| Error::Expired)?
}

/// Waits for Sheprd to close the input, dropping anything meanwhile.
async fn closed(frames: &mut mpsc::Receiver<Input>) {
    while frames.recv().await.is_some() {}
}

/// The state directory a `desktop-pair` or `relay-connect` uses.
pub(crate) fn state_dir(given: Option<&Path>) -> Option<PathBuf> {
    given
        .map(Path::to_owned)
        .or_else(crate::app::enroll::default_state_dir)
}
