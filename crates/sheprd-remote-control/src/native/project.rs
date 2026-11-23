//! New projects for the native app (`POST /api/v2/projects`): a folder in
//! one of the project roots `GET /api/v2/new` offers, either an empty Git
//! repository on `main` or a clone. The phone then opens a workspace in it
//! with `POST /api/v2/workspaces`, as for any folder there.
//!
//! Git runs in a hidden folder beside the new one, which is renamed into
//! place only once it is whole, so a clone that fails or is stopped never
//! leaves a half project under the name, and nothing already there is ever
//! removed.

mod error;

pub(crate) use error::Error;

use crate::web::{ApiError, Device, Shared, folders};
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
use tokio::sync::Semaphore;

/// How long a clone may run. The phone waits for it, so this is also about
/// how long its request stays open.
pub(crate) const CLONE_WAIT: Duration = Duration::from_secs(240);
/// How long `git init` may run.
const INIT_WAIT: Duration = Duration::from_secs(15);
/// Projects being made at once, across every phone.
static MAKING: Semaphore = Semaphore::const_new(2);
/// The most of Git's error output kept, from its end.
const MAX_OUTPUT: usize = 600;
/// The longest folder name and repository address accepted.
const MAX_NAME: usize = 100;
const MAX_URL: usize = 2048;
/// The protocols a clone may use; Git's `ext::` and local paths are not.
const ALLOWED_PROTOCOLS: &str = "https:ssh";

#[derive(Debug, Deserialize)]
pub(crate) struct NewProject {
    /// A repository to clone: `owner/name` for GitHub, or an `https://`,
    /// `ssh://`, or `user@host:path` address. Absent makes an empty one.
    #[serde(default)]
    clone: Option<String>,
    name: String,
    /// One of the project roots `GET /api/v2/new` lists.
    root: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct CreatedProject {
    pub(crate) path: String,
}

/// `POST /api/v2/projects`.
pub(crate) async fn create_project(
    State(state): State<Shared>,
    _device: Device,
    Json(request): Json<NewProject>,
) -> Result<Json<CreatedProject>, ApiError> {
    let name = check_name(&request.name)?.to_owned();
    let source = request.clone.as_deref().map(clone_url).transpose()?;
    let layout = state.hub.layout();
    let root = request.root;
    let (root, name) = tokio::task::spawn_blocking(move || {
        if !folders(&layout)
            .roots
            .iter()
            .any(|(offered, _)| *offered == root)
        {
            return Err(Error::Root);
        }
        vacant(Path::new(&root), &name)?;
        Ok((root, name))
    })
    .await
    .map_err(|_| Error::Worker)??;
    // Taken only once the request could succeed, so a refused one never
    // holds up another phone's.
    let making = MAKING.try_acquire().map_err(|_| Error::Busy)?;
    let path = tokio::task::spawn_blocking(move || {
        // The worker owns the slot even if the HTTP request is cancelled.
        let _making = making;
        make(Path::new(&root), &name, source.as_deref())
    })
    .await
    .map_err(|_| Error::Worker)??;
    let path = path.to_str().ok_or(Error::Root)?.to_owned();
    Ok(Json(CreatedProject { path }))
}

/// The folder name, trimmed: one plain path segment that is not hidden and
/// cannot be read as an option.
pub(crate) fn check_name(name: &str) -> Result<&str, Error> {
    let name = name.trim();
    let plain = !name.is_empty()
        && name.chars().count() <= MAX_NAME
        && !name.starts_with(['.', '-'])
        && !name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'));
    plain.then_some(name).ok_or(Error::Name)
}

/// The address Git is given: `owner/name` is GitHub's, over HTTPS; any
/// other must be an `https://` or `ssh://` URL or scp-like `user@host:path`.
pub(crate) fn clone_url(address: &str) -> Result<String, Error> {
    let address = address.trim();
    if address.is_empty()
        || address.len() > MAX_URL
        || address.starts_with('-')
        || address.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err(Error::Url);
    }
    if let Some((owner, repo)) = address.split_once('/')
        && !address.contains(':')
        && [owner, repo].iter().all(|part| github_segment(part))
    {
        return Ok(format!("https://github.com/{owner}/{repo}.git"));
    }
    if let Some(rest) = address
        .strip_prefix("https://")
        .or_else(|| address.strip_prefix("ssh://"))
    {
        return rest
            .split('/')
            .next()
            .is_some_and(|host| !host.is_empty())
            .then(|| address.to_owned())
            .ok_or(Error::Url);
    }
    // scp-like: a user and host before the first colon, which a URL would
    // follow with `//`.
    match address.split_once(':') {
        Some((login, path))
            if login.contains('@')
                && !login.contains('/')
                && !path.is_empty()
                && !path.starts_with('/') =>
        {
            Ok(address.to_owned())
        }
        _ => Err(Error::Url),
    }
}

fn github_segment(part: &str) -> bool {
    !part.is_empty()
        && part.len() <= MAX_NAME
        && !matches!(part, "." | "..")
        && part
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Makes `root/name`: Git works in a hidden folder in `root`, which is
/// renamed to the name once it is done. Blocking.
fn make(root: &Path, name: &str, source: Option<&str>) -> Result<PathBuf, Error> {
    let target = vacant(root, name)?;
    let scratch = tempfile::Builder::new()
        .prefix(".sheprd-new-")
        .tempdir_in(root)
        .map_err(Error::Place)?;
    let work = scratch.path().join(name);
    match source {
        Some(url) => git(
            scratch.path(),
            &["clone", "--quiet", "--", url, name],
            CLONE_WAIT,
        )?,
        None => {
            std::fs::create_dir(&work).map_err(Error::Place)?;
            git(
                &work,
                &["init", "--quiet", "--initial-branch=main"],
                INIT_WAIT,
            )?;
        }
    }
    sheprd_client::filesystem::rename_noreplace(&work, &target).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            Error::Exists {
                name: name.to_owned(),
            }
        } else {
            Error::Place(error)
        }
    })?;
    Ok(target)
}

/// `root/name`, unless something is already there. Blocking.
fn vacant(root: &Path, name: &str) -> Result<PathBuf, Error> {
    let target = root.join(name);
    match target.symlink_metadata() {
        Ok(_) => Err(Error::Exists {
            name: name.to_owned(),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(target),
        Err(error) => Err(Error::Place(error)),
    }
}

/// Runs Git in `dir` for at most `wait`, without a terminal to ask for a
/// password on and with only the allowed protocols. Blocking.
fn git(dir: &Path, args: &[&str], wait: Duration) -> Result<(), Error> {
    let mut command = sheprd_client::new_command("git");
    // A user service's `PATH` is bare; the login shell's finds the user's Git.
    sheprd_client::login_env::apply(&mut command);
    command
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ALLOW_PROTOCOL", ALLOWED_PROTOCOLS)
        .env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes");
    run(command, wait)
}

/// Reuse the script runner's process group / Windows job and pipe cleanup.
/// Its idle limit alone is not a wall-clock deadline when Git is chatty.
fn run(command: Command, wait: Duration) -> Result<(), Error> {
    let cancelled = AtomicBool::new(false);
    let (done, finished) = mpsc::channel();
    let result = std::thread::scope(|scope| {
        let cancelled = &cancelled;
        scope.spawn(move || {
            if matches!(
                finished.recv_timeout(wait),
                Err(mpsc::RecvTimeoutError::Timeout)
            ) {
                cancelled.store(true, Ordering::Release);
            }
        });
        let result = sheprd_client::run_script_command(
            command,
            std::io::empty(),
            std::io::sink(),
            sheprd_client::ScriptLimits {
                output: u64::MAX,
                idle: wait,
            },
            cancelled,
        );
        let _ = done.send(());
        result
    });
    match result {
        Ok(_) => Ok(()),
        Err(sheprd_client::Error::ScriptCancelled | sheprd_client::Error::ScriptTimeout) => {
            Err(Error::TimedOut)
        }
        Err(sheprd_client::Error::ScriptExit { stderr, .. }) => Err(Error::Refused {
            output: last_lines(stderr.as_bytes()),
        }),
        Err(error) => Err(Error::Git(error)),
    }
}

/// Git's last words, at most [`MAX_OUTPUT`] bytes on a character boundary.
pub(crate) fn last_lines(output: &[u8]) -> String {
    let text = String::from_utf8_lossy(output);
    let text = text.trim();
    let mut start = text.len().saturating_sub(MAX_OUTPUT);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    text[start..].trim().to_owned()
}

#[cfg(test)]
mod tests;
