# Repository visibility across sidebar layouts

Native macOS GPUI renderer captures for PR #1549, using the same synthetic
repositories, worktrees, selection, font, window size, and themes in both builds.
These are actual application frames, not HTML mockups or generated images.

- **Before:** `4d2b3fd82`, the parent of the sidebar visibility change.
- **After:** `84f758688`, with an unused local variable removed from `row.rs`
  during the capture build (no visual effect).
- The two `after/rail-*.png` captures are refreshed with the divider-spacing
  follow-up in their containing commit: each divider now sits between rows,
  with 6 points of clearance on each side, rather than against the next tile.
- Compact and Minimal are captured together from `a86449466` for the after
  comparison, against the same before revision. Compact means the expanded
  Classic layout with compact density and flat styling, not the collapsed rail.
- **Window:** 460 × 700 points, captured at 2× resolution (920 × 1400 pixels).
  The normal sidebar width clamp remains active at this narrow window size.
- **Font:** Menlo, 14 points. **Themes:** Catppuccin Latte and Catppuccin Mocha.
- Four repositories have no avatar. Their linked worktrees stay grouped;
  `sidebar polish` is selected in both builds. No real terminal content,
  saved credentials, or personal session data is included.

## Comparisons

| Layout | Before, light | After, light | Before, dark | After, dark |
| --- | --- | --- | --- | --- |
| Classic | [PNG](before/classic-light.png) | [PNG](after/classic-light.png) | [PNG](before/classic-dark.png) | [PNG](after/classic-dark.png) |
| Compact | [PNG](before/compact-light.png) | [PNG](after/compact-light.png) | [PNG](before/compact-dark.png) | [PNG](after/compact-dark.png) |
| Superset | [PNG](before/superset-light.png) | [PNG](after/superset-light.png) | [PNG](before/superset-dark.png) | [PNG](after/superset-dark.png) |
| Orca | [PNG](before/orca-light.png) | [PNG](after/orca-light.png) | [PNG](before/orca-dark.png) | [PNG](after/orca-dark.png) |
| Minimal | [PNG](before/minimal-light.png) | [PNG](after/minimal-light.png) | [PNG](before/minimal-dark.png) | [PNG](after/minimal-dark.png) |
| Collapsed rail | [PNG](before/rail-light.png) | [PNG](after/rail-light.png) | [PNG](before/rail-dark.png) | [PNG](after/rail-dark.png) |

## Capture method and limits

Both versions were built in a temporary source directory with
`cargo build --locked -p sheprd-gpui --no-default-features --features integration-test,mockup`.
A temporary `--sidebar-test` driver supplied synthetic snapshot state, cycled
the layouts and two themes, and saved `Window::render_to_image` frames.
Each process used an empty, isolated home and config directories and connected
to no daemon. The capture driver is not part of the application changes.

The capture-only build also shortened an existing redundant `HashMap`
qualification in `settings_window/controls/fonts.rs` to get past that
integration-test build lint. This identical, nonvisual adjustment was applied
to both captures and is not included in this PR.

The screenshots demonstrate missing-avatar fallback, grouping, selection, and
narrow-window presentation in the classic Rust window. They do not verify
the Moonframe-hosted layout, live avatar loading or failure, drag interactions,
custom layouts, large fonts, or the full Rust test suite.
