//! Which machine an index describes. The id is derived from the machine's
//! own identity, read-only, so every Sheprd that reaches the same machine
//! (itself, or another over SSH) names it the same way and the same session
//! is never kept twice. Hostnames change and collide, so they are labels only.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};
use uuid::Uuid;

/// Files that hold a stable machine id on Linux and BSD.
const MACHINE_ID: [&str; 2] = ["/etc/machine-id", "/var/lib/dbus/machine-id"];
/// The id file kept when the machine offers none.
const FALLBACK: &str = "host-id";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Host {
    pub id: String,
    pub label: String,
    pub os: String,
    /// `user@label`: whose agents these are.
    pub user: String,
}

impl Host {
    /// This machine, keeping a fallback id in `state` when it has no
    /// machine id of its own.
    pub fn local(state: &Path) -> Result<Self> {
        let label = hostname();
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "me".to_owned());
        Ok(Self {
            id: id(machine_id(), state)?,
            user: format!("{user}@{label}"),
            label,
            os: std::env::consts::OS.into(),
        })
    }

    /// The host id for a machine whose own id reads `machine_id`.
    pub fn id_of(machine_id: &str) -> String {
        Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("sheprd-traces:{}", machine_id.trim()).as_bytes(),
        )
        .hyphenated()
        .to_string()
    }
}

fn id(machine: Option<String>, state: &Path) -> Result<String> {
    if let Some(machine) = machine {
        return Ok(Host::id_of(&machine));
    }
    let path = state.join(FALLBACK);
    match fs::read_to_string(&path) {
        Ok(saved) if Uuid::parse_str(saved.trim()).is_ok() => return Ok(saved.trim().to_owned()),
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(source) => return Err(Error::Io { path, source }),
    }
    let id = Uuid::new_v4().hyphenated().to_string();
    sheprd_client::private_file::prepare(&path)?;
    fs::write(&path, &id).map_err(|source| Error::Io { path, source })?;
    Ok(id)
}

fn machine_id() -> Option<String> {
    MACHINE_ID
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .map(|id| id.trim().to_owned())
        .find(|id| !id.is_empty())
        .or_else(platform_uuid)
}

/// macOS's hardware UUID, which `ioreg` prints as `"IOPlatformUUID" = "…"`.
#[cfg(target_os = "macos")]
fn platform_uuid() -> Option<String> {
    let output = sheprd_client::new_command("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let line = text.lines().find(|line| line.contains("IOPlatformUUID"))?;
    let value = line.rsplit('=').next()?.trim().trim_matches('"');
    (!value.is_empty()).then(|| value.to_owned())
}

#[cfg(not(target_os = "macos"))]
fn platform_uuid() -> Option<String> {
    None
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .or_else(|| fs::read_to_string("/etc/hostname").ok())
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .or_else(|| {
            let output = sheprd_client::new_command("hostname").output().ok()?;
            let name = String::from_utf8(output.stdout).ok()?.trim().to_owned();
            (!name.is_empty()).then_some(name)
        })
        .unwrap_or_else(|| "localhost".to_owned())
}
