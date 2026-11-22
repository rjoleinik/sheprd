//! Finds the agents' session files on this host and indexes what each
//! gained since the last scan. Only complete lines are read; a partial last
//! line waits for the next scan. A replaced or shrunken file is read again
//! from the start.
use crate::{
    Error, Host, Result, Store,
    parse::Parser,
    redact::Redactor,
    store::{Cursor, Update},
};
use serde::Serialize;
use sheprd_client::herdr::AgentKind;
use std::{
    fs::{self, Metadata},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};
use uuid::Uuid;

mod open;

/// The most bytes read from one file in one scan; the rest waits.
const CHUNK: u64 = 32 * 1024 * 1024;
/// Longer records (pasted images, huge tool output) are skipped whole.
const MAX_RECORD: usize = 4 * 1024 * 1024;
/// How deep below a root session files are looked for.
const DEPTH: usize = 4;
/// The most session files one root yields.
const MAX_FILES: usize = 20_000;

/// A directory where one agent keeps its session files.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Root {
    pub agent: AgentKind,
    pub dir: PathBuf,
}

/// What a scan did.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Report {
    pub files: usize,
    pub changed: usize,
    pub events: usize,
    /// Files that could not be read, with why.
    pub skipped: Vec<String>,
    /// The sessions that changed, each once, in the order first changed.
    pub sessions: Vec<String>,
}

impl Report {
    fn add(&mut self, outcome: Result<Option<(String, usize)>>) -> Result<()> {
        match outcome {
            Ok(Some((session, added))) => {
                self.changed += 1;
                self.events += added;
                if !self.sessions.contains(&session) {
                    self.sessions.push(session);
                }
            }
            Ok(None) => {}
            Err(Error::Io { path, source }) => {
                self.skipped.push(format!("{}: {source}", path.display()));
            }
            Err(error) => return Err(error),
        }
        Ok(())
    }
}

/// Indexes every session file under `roots` into `store` as `host`'s, at
/// `now` (Unix milliseconds).
pub fn index(
    store: &mut Store,
    host: &Host,
    roots: &[Root],
    redactor: &Redactor,
    now: i64,
) -> Result<Report> {
    store.host(host, "local", now)?;
    let mut report = Report::default();
    for root in roots {
        let mut files = Vec::new();
        walk(&root.dir, DEPTH, &mut files);
        files.sort();
        for path in files {
            report.files += 1;
            report.add(file(store, host, root, &path, redactor))?;
        }
    }
    Ok(report)
}

/// Indexes only `paths`, which a watcher reported changed. Paths outside
/// `roots`, too deep, or not session files are ignored; a path that no
/// longer exists is left as last indexed.
pub fn index_paths(
    store: &mut Store,
    host: &Host,
    roots: &[Root],
    paths: &[PathBuf],
    redactor: &Redactor,
) -> Result<Report> {
    let mut report = Report::default();
    for path in paths {
        let Some(root) = roots.iter().find(|root| session_file(&root.dir, path)) else {
            continue;
        };
        report.files += 1;
        report.add(file(store, host, root, path, redactor))?;
    }
    Ok(report)
}

/// Whether `path` has a permitted session-file spelling beneath `root`.
/// File types and links are checked while opening its components.
pub(crate) fn session_file(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let parts = relative.components().count();
    parts > 0
        && parts <= DEPTH + 1
        && path.extension().is_some_and(|ext| ext == "jsonl")
        && relative
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(name) if name != "subagents"))
}

/// Indexes what `path` gained: its session and how many events were
/// added, or `None` when it is unchanged.
fn file(
    store: &mut Store,
    host: &Host,
    root: &Root,
    path: &Path,
    redactor: &Redactor,
) -> Result<Option<(String, usize)>> {
    let io = |source| Error::Io {
        path: path.to_owned(),
        source,
    };
    let Some(mut handle) = open::file(&root.dir, path).map_err(io)? else {
        return Ok(None);
    };
    let agent = root.agent;
    let metadata = handle.metadata().map_err(io)?;
    let ident = ident(&metadata);
    let key = path.to_string_lossy();
    let saved = store.cursor(&host.id, &key)?;
    let restart = saved
        .as_ref()
        .is_some_and(|cursor| cursor.ident != ident || cursor.offset > metadata.len());
    let mut cursor = match saved {
        Some(cursor) if !restart => cursor,
        _ => Cursor {
            session_id: session_id(&host.id, agent, path),
            ident,
            offset: 0,
            parser: Parser::new(),
        },
    };
    if cursor.offset == metadata.len() && !restart {
        return Ok(None);
    }
    handle.seek(SeekFrom::Start(cursor.offset)).map_err(io)?;
    let mut bytes = Vec::new();
    handle
        .take(CHUNK.min(metadata.len() - cursor.offset))
        .read_to_end(&mut bytes)
        .map_err(io)?;
    // Only complete lines; a record still being written waits.
    let Some(end) = bytes.iter().rposition(|byte| *byte == b'\n') else {
        if bytes.len() as u64 >= CHUNK {
            // One record larger than a whole chunk: skip past it.
            cursor.offset += bytes.len() as u64;
        }
        let added = persist(store, host, agent, &key, &cursor, &[], restart)?;
        return Ok(Some((cursor.session_id, added)));
    };
    let mut events = Vec::new();
    let mut offset = cursor.offset;
    for line in bytes[..=end].split(|byte| *byte == b'\n') {
        let start = offset;
        offset += line.len() as u64 + 1;
        if line.is_empty() || line.len() > MAX_RECORD {
            continue;
        }
        if let Ok(line) = std::str::from_utf8(line) {
            cursor
                .parser
                .record(agent, start, line, redactor, &mut events);
        }
    }
    // The split yields one empty piece after the final newline.
    cursor.offset = offset - 1;
    let added = persist(store, host, agent, &key, &cursor, &events, restart)?;
    Ok(Some((cursor.session_id, added)))
}

fn persist(
    store: &mut Store,
    host: &Host,
    agent: AgentKind,
    path: &str,
    cursor: &Cursor,
    events: &[crate::Event],
    restart: bool,
) -> Result<usize> {
    store.apply(&Update {
        host,
        agent,
        path,
        cursor,
        events,
        restart,
    })
}

/// A session's id: the same for the same file of the same agent on the
/// same host, whichever Sheprd indexed it.
fn session_id(host_id: &str, agent: AgentKind, path: &Path) -> String {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("{host_id}/{}/{stem}", agent.name()).as_bytes(),
    )
    .hyphenated()
    .to_string()
}

#[cfg(unix)]
fn ident(metadata: &Metadata) -> String {
    use std::os::unix::fs::MetadataExt;
    format!("{}:{}", metadata.dev(), metadata.ino())
}

#[cfg(not(unix))]
fn ident(metadata: &Metadata) -> String {
    // Without inodes, a file is known by when it was created.
    metadata
        .created()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or_else(String::new, |time| time.as_nanos().to_string())
}

/// Collects `*.jsonl` files under `dir`, `depth` levels down, never
/// following links.
fn walk(dir: &Path, depth: usize, files: &mut Vec<PathBuf>) {
    if !open::directory(dir) {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if files.len() >= MAX_FILES {
            return;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        // Claude keeps a session's subagent runs beside it; they belong to
        // the parent session's tool calls, not to sessions of their own.
        if kind.is_dir() && depth > 0 && entry.file_name() != "subagents" {
            walk(&path, depth - 1, files);
        } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "jsonl") {
            files.push(path);
        }
    }
}
