//! Counts against the branch's configured upstream, without fetching.

use std::time::Instant;

/// Ahead and behind, or none when the branch has no usable upstream.
pub fn read(
    dir: &str,
    deadline: Instant,
    cancelled: &impl Fn() -> bool,
) -> crate::Result<Option<(usize, usize)>> {
    let output = match crate::git::git(
        dir,
        &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
        "count upstream commits",
        deadline,
        cancelled,
    ) {
        Ok(output) => output,
        Err(crate::Error::Git(crate::git::Error::Failed { .. })) => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut counts = output.split_whitespace();
    let parsed = counts
        .next()
        .and_then(|ahead| ahead.parse().ok())
        .zip(counts.next().and_then(|behind| behind.parse().ok()));
    Ok(parsed.filter(|_| counts.next().is_none()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
