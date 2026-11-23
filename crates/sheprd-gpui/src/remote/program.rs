//! Finding a machine's `sheprd-remote-control` over SSH, without installing
//! it, for every script Sheprd runs against another machine's daemon.

/// The `/bin/sh` lines that set `program` to the machine's daemon program,
/// or to a path that is not executable when it has none: the one on `PATH`
/// under either name, then the one Sheprd installs in `~/.local/bin`, then
/// the copy inside a macOS app bundle. A Mac that installed Sheprd from its
/// disk image has only that copy, which never joins `PATH`, so without it a
/// computer running Sheprd reads as a machine with none. Every copy reads
/// the same state directory, so whichever is found reaches the same daemon.
/// A literal, so callers `concat!` it before their own lines.
macro_rules! find_program {
    () => {
        // i18n: data
        "program=$(command -v sheprd-remote-control || command -v herdr-remote-control || true)
for candidate in \"$HOME/.local/bin/sheprd-remote-control\" \"$HOME/.local/bin/herdr-remote-control\" \"$HOME/Applications/Sheprd.app/Contents/MacOS/sheprd-remote-control\" /Applications/Sheprd.app/Contents/MacOS/sheprd-remote-control \"$HOME/Applications/Herdr.app/Contents/MacOS/herdr-remote-control\" /Applications/Herdr.app/Contents/MacOS/herdr-remote-control; do
  [ -n \"$program\" ] && [ -x \"$program\" ] && break
  program=$candidate
done
"
    };
}
pub(crate) use find_program;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
