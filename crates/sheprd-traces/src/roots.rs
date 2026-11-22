//! Pure profile-path resolution. Callers supply the environment their agents
//! inherit; a desktop supplies its resolved login environment on its worker.
use crate::Root;
use sheprd_client::herdr::AgentKind;
use std::{collections::HashSet, ffi::OsString, path::Path};

/// Default locations, without reading the process environment.
pub fn default_roots(home: &Path) -> Vec<Root> {
    configured_roots(home, home, |_| None)
}

/// Configured locations, with relative direct overrides resolved from `cwd`.
/// Empty overrides mean the default, as the history loader treats them.
pub fn configured_roots(
    home: &Path,
    cwd: &Path,
    env: impl Fn(&str) -> Option<OsString>,
) -> Vec<Root> {
    let env = |name| env(name).filter(|value| !value.is_empty());
    let directory =
        |name, default: &str| env(name).map_or_else(|| home.join(default), |path| cwd.join(path));
    let claude = directory("CLAUDE_CONFIG_DIR", ".claude");
    let codex = directory("CODEX_HOME", ".codex");
    let pi = directory("PI_CODING_AGENT_DIR", ".pi/agent");
    let omp = env("PI_CODING_AGENT_DIR").map_or_else(
        || {
            home.join(env("PI_CONFIG_DIR").unwrap_or_else(|| ".omp".into()))
                .join("agent")
        },
        |path| cwd.join(path),
    );
    let mut roots = vec![
        Root {
            agent: AgentKind::Claude,
            dir: claude.join("projects"),
        },
        Root {
            agent: AgentKind::Codex,
            dir: codex.join("sessions"),
        },
        Root {
            agent: AgentKind::Codex,
            dir: codex.join("archived_sessions"),
        },
        Root {
            agent: AgentKind::Pi,
            dir: pi.join("sessions"),
        },
        Root {
            agent: AgentKind::Omp,
            dir: omp.join("sessions"),
        },
    ];
    // Pi and omp share their override names and file format. A shared custom
    // directory is read once with the Pi-compatible parser, not double-counted.
    // Retain omp's identity when that shared directory is its standard location.
    let omp = home.join(".omp/agent/sessions");
    for root in &mut roots {
        if root.agent == AgentKind::Pi && root.dir == omp {
            root.agent = AgentKind::Omp;
        }
    }
    let mut seen = HashSet::new();
    roots.retain(|root| seen.insert(root.dir.clone()));
    roots
}
