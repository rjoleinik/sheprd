# Traces native captures

Captured from this branch's native Linux/X11 window using the isolated
`native_traces` fixture. All session content is synthetic. These are native
app captures, not browser mockups; no personal transcripts appear here.
The disconnected status bar belongs to the daemon-free fixture.
The current captures include synthetic replication status for two connected
hosts and cached sessions from three computers. They demonstrate presentation,
not a real network exchange; the replication tests exercise the exchange.

| View | Capture |
| --- | --- |
| Dark, 1400 px | ![Dark Traces dashboard](dark.png) |
| Light, 1400 px | ![Light Traces dashboard](light.png) |
| Dark, 640 px | ![Narrow dark Traces dashboard](narrow-dark.png) |
| Light, 640 px | ![Narrow light Traces dashboard](narrow-light.png) |
| Selected session (captured before the host-filter controls were added) | ![Session details](detail.png) |
| Work label: two computers | ![Traces filtered to work-labelled hosts](tag-work.png) |
| One computer | ![Traces filtered to Workstation](host-workstation.png) |

## Narrow layout correction

The first capture exposed table columns clipped without a horizontal scroll
container. The final layout keeps the table in a horizontally scrollable
viewport and wraps the detail panel below it when needed.

| Before the layout correction | After the layout correction |
| --- | --- |
| ![Clipped table before the fix](narrow-before.png) | ![Scrollable table after the fix](narrow-dark.png) |

The comparison is before/after the narrow-table fix within this branch, not a
capture of `main`: Traces is a new page with no equivalent on `main`.
See [Traces](../../traces.md#native-screenshots) for the reproduction command.
