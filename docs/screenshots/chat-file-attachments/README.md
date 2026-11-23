# Native file attachment cards

These macOS captures show the native chat renderer with a synthetic agent
reply containing standalone links to a real synthetic video and PDF. Their
Quick Look thumbnails show a video frame and the first PDF page. No personal
session or file contents are included. The compact cards have rounded corners,
rounded preview edges, neutral filename labels, and a separate Open button.

- [Dark theme](dark.png)
- [Light theme](light.png)
- [Narrow window](narrow.png)

The opt-in `files` fixture uses the real transcript parser, card layout, and
native painter. Build with:

```sh
cargo build --locked -p sheprd-gpui --bin sheprd --features integration-test,mockup
```

Run the isolated fixture with `HERDR_TEST_RICH_CHAT=files`,
`HERDR_TEST_RICH_CHAT_CAPTURE=/absolute/output.png`, and `--sidebar-test`.
Set `HERDR_TEST_RICH_CHAT_THEME=light` for light mode, or
`HERDR_TEST_RICH_CHAT_SIZE=narrow` for the narrow window. Use private
`XDG_CONFIG_HOME` and `XDG_STATE_HOME` directories when capturing.

These screenshots verify the visible file cards, native thumbnails, and Open
controls. They do not exercise external media/PDF applications or attachment
delivery to a live agent. The same sample conversation before the card polish is
available in the [previous native capture](https://github.com/sheprd-labs/sheprd/blob/810fb82f2c522075cc48ad3f74afc734a4f38e10/docs/screenshots/chat-file-attachments/dark.png).
No VM or installed app was changed.

Thumbnails are generated only for local PDF/video attachments on macOS.
Missing/unsupported files and remote attachments retain a generic file icon.
Visible thumbnails are checked in the background every two seconds. Replaced
or edited files regenerate their preview; unchanged files reuse their cached
image, and missing files are checked again so newly written attachments appear.
The native tests use the actual Quick Look service, including overwriting a PDF
at the same path and checking that the rendered pixels change:

```sh
cargo test --locked -p sheprd-gpui --lib chat::visual::thumbnail::tests::native_ -- --ignored
```
