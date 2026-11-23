//! A Live Activity's step in words, for the phone's Standard and Complete
//! styles, beside the step itself, which the Geek style shows. Agents title
//! a shell call with the command as they ran it, such as
//! `` `/bin/zsh -lc "gh pr checks 1548"` ``: [`clean`] takes the wrapper
//! and quotes off, and [`describe`] says what the call does from its kind
//! and, for a command, the command's first words. Rules only: a command no
//! rule knows is "Running a command".

use super::payload::Icon;
use crate::push::excerpt;
use sheprd_chat_protocol::{Tool, ToolKind};
use std::path::Path;

/// The step in plain words and its icon.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Described {
    pub(crate) activity: String,
    pub(crate) icon: Icon,
}

impl Described {
    fn new(activity: impl Into<String>, icon: Icon) -> Self {
        Self {
            activity: activity.into(),
            icon,
        }
    }
}

/// `title` with the backticks an agent puts around a command and a shell
/// wrapper (`sh -c`, `bash -lc`, `/bin/zsh -lc`, …) and its quotes taken off.
pub(crate) fn clean(title: &str) -> String {
    let mut text = title.trim();
    text = strip_pair(text, '`');
    let mut words = text.splitn(3, char::is_whitespace);
    let (Some(shell), Some(flag), Some(rest)) = (words.next(), words.next(), words.next()) else {
        return text.to_owned();
    };
    let shell = shell.rsplit('/').next().unwrap_or(shell);
    let wraps = matches!(shell, "sh" | "bash" | "zsh" | "fish" | "dash")
        && flag.starts_with('-')
        && flag.ends_with('c');
    if !wraps {
        return text.to_owned();
    }
    let rest = rest.trim();
    strip_pair(strip_pair(rest, '"'), '\'').trim().to_owned()
}

fn strip_pair(text: &str, quote: char) -> &str {
    text.strip_prefix(quote)
        .and_then(|inner| inner.strip_suffix(quote))
        .unwrap_or(text)
}

/// What `tool` does, in words.
pub(crate) fn describe(tool: &Tool) -> Described {
    let file = tool.locations.first().map(|path| file_name(path));
    let on = |verb: &str, icon| match &file {
        Some(file) => Described::new(excerpt(&format!("{verb} {file}")), icon),
        None => Described::new(format!("{verb} files"), icon),
    };
    match tool.kind {
        ToolKind::Execute => command(&clean(&tool.title)),
        ToolKind::Edit => on("Editing", Icon::Edit),
        ToolKind::Delete => on("Deleting", Icon::Edit),
        ToolKind::Move => Described::new("Moving files", Icon::Edit),
        ToolKind::Read => on("Reading", Icon::Read),
        ToolKind::Search => Described::new("Searching the code", Icon::Search),
        ToolKind::Fetch => Described::new("Looking something up", Icon::Web),
        ToolKind::Think => Described::new("Thinking", Icon::Think),
        ToolKind::Subagent => Described::new("Working with a sub-agent", Icon::Agent),
        ToolKind::SwitchMode => Described::new("Switching mode", Icon::Think),
        // A tool with no kind may still be a command, as some agents send.
        ToolKind::Other => {
            let cleaned = clean(&tool.title);
            if cleaned == tool.title.trim() {
                Described::new(excerpt(&cleaned), Icon::Run)
            } else {
                command(&cleaned)
            }
        }
    }
}

/// A plan step in progress, which the agent already wrote in words.
pub(crate) fn planned(content: &str) -> Described {
    Described::new(excerpt(content), Icon::Plan)
}

fn file_name(path: &str) -> String {
    Path::new(path).file_name().map_or_else(
        || path.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// What a shell command does. Of a sequence (`a && b; c`) the last command
/// counts, as the earlier ones usually set it up; of a pipeline, the first.
fn command(line: &str) -> Described {
    let sequence = line.replace("&&", ";").replace("||", ";");
    let last = sequence
        .split(';')
        .map(str::trim)
        .rfind(|part| !part.is_empty())
        .unwrap_or(line);
    let first = last.split('|').next().unwrap_or(last);
    let words: Vec<&str> = first
        .split_whitespace()
        // Environment assignments in front name no program.
        .skip_while(|word| word.contains('=') && !word.starts_with('-'))
        .collect();
    let program = words
        .first()
        .map_or("", |word| word.rsplit('/').next().unwrap_or(word));
    let sub = |index: usize| words.get(index).copied().unwrap_or("");
    let number = words.iter().find(|word| word.parse::<u32>().is_ok());
    match (program, sub(1), sub(2)) {
        ("gh", "pr", "checks") => pull_request("Checking pull request", number, Icon::Github),
        ("gh", "pr", "view" | "diff") => pull_request("Reading pull request", number, Icon::Github),
        ("gh", "pr", "create") => Described::new("Opening a pull request", Icon::Github),
        ("gh", "pr", "merge") => pull_request("Merging pull request", number, Icon::Github),
        ("gh", "pr", "comment" | "review") => {
            pull_request("Commenting on pull request", number, Icon::Github)
        }
        ("gh", "run", _) => Described::new("Checking CI", Icon::Github),
        ("gh", "issue", _) => Described::new("Reading issues", Icon::Github),
        ("gh", _, _) => Described::new("Asking GitHub", Icon::Github),
        ("git", "status" | "diff" | "show", _) => {
            Described::new("Looking at the changes", Icon::Git)
        }
        ("git", "log" | "blame", _) => Described::new("Reading the history", Icon::Git),
        ("git", "add", _) => Described::new("Staging changes", Icon::Git),
        ("git", "commit", _) => Described::new("Committing", Icon::Git),
        ("git", "push", _) => Described::new("Pushing", Icon::Git),
        ("git", "pull" | "fetch", _) => Described::new("Getting the latest changes", Icon::Git),
        ("git", "checkout" | "switch" | "branch" | "worktree", _) => {
            Described::new("Switching branches", Icon::Git)
        }
        ("git", "rebase" | "merge" | "cherry-pick", _) => {
            Described::new("Combining branches", Icon::Git)
        }
        ("git", _, _) => Described::new("Working with Git", Icon::Git),
        ("cargo", "test" | "nextest", _)
        | ("swift" | "go", "test", _)
        | ("pytest" | "jest" | "vitest", _, _)
        | ("npm" | "pnpm" | "yarn" | "bun", "test", _) => {
            Described::new("Running tests", Icon::Test)
        }
        ("just", recipe, _) if recipe.starts_with("test") => {
            Described::new("Running tests", Icon::Test)
        }
        ("just", "ci" | "lint" | "check" | "fmt", _) | ("cargo", "clippy" | "fmt", _) => {
            Described::new("Running checks", Icon::Test)
        }
        ("cargo" | "swift" | "go", "build" | "check" | "run", _)
        | ("xcodebuild" | "make" | "cmake" | "ninja" | "tsc", _, _)
        | ("npm" | "pnpm" | "yarn" | "bun", "run" | "build", _) => {
            Described::new("Building", Icon::Build)
        }
        ("npm" | "pnpm" | "yarn" | "bun" | "pip" | "brew", "install" | "add" | "i", _) => {
            Described::new("Installing packages", Icon::Build)
        }
        ("rg" | "grep" | "fd" | "find" | "ag", _, _) => {
            Described::new("Searching the code", Icon::Search)
        }
        ("cat" | "sed" | "head" | "tail" | "less" | "bat" | "nl" | "wc", _, _) => {
            Described::new("Reading files", Icon::Read)
        }
        ("ls" | "tree" | "pwd", _, _) => Described::new("Looking around", Icon::Read),
        ("curl" | "wget" | "http", _, _) => Described::new("Fetching from the web", Icon::Web),
        _ => Described::new("Running a command", Icon::Run),
    }
}

fn pull_request(verb: &str, number: Option<&&str>, icon: Icon) -> Described {
    match number {
        Some(number) => Described::new(format!("{verb} #{number}"), icon),
        None => Described::new(verb.to_owned(), icon),
    }
}
