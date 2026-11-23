use crate::codex::bridge::tests::{THREAD, harness, opened, request};
use crate::files::{Accepts, prepare};
use crate::harness::Translator as _;
use serde_json::json;
use sheprd_chat_protocol::{Attachment, Command};

#[test]
fn video_attachments_reach_codex_as_readable_paths_with_or_without_a_caption() {
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "Sheprd — Home (Window) 08.29 PM.mp4",
        "clip #1%.mov",
        "clip.webm",
    ] {
        let path = dir.path().join(name);
        // Binary contents force the same resource-link fallback as a video.
        std::fs::write(&path, b"\0\0\0\x18ftypmp42").unwrap();
        for caption in ["Inspect this video", ""] {
            let command = Command::Prompt {
                text: caption.into(),
                attachments: vec![Attachment {
                    path: path.display().to_string(),
                }],
                origin: None,
            };
            // The native composer sends paths through the attach wire, not bytes.
            let command: Command =
                serde_json::from_slice(&serde_json::to_vec(&command).unwrap()).unwrap();
            let Command::Prompt {
                text, attachments, ..
            } = command
            else {
                panic!("expected prompt");
            };
            let mut blocks = Vec::new();
            if !text.is_empty() {
                blocks.push(acp::v1::ContentBlock::Text(acp::v1::TextContent::new(text)));
            }
            let prepared = prepare(
                &attachments[0],
                Accepts {
                    embedded: true,
                    image: true,
                },
            );
            assert!(matches!(
                prepared.block,
                acp::v1::ContentBlock::ResourceLink(_)
            ));
            assert!(prepared.image.is_none());
            blocks.push(prepared.block);
            let mut bridge = opened();
            let outs = bridge.on_session(request(
                2,
                "session/prompt",
                json!({
                    "sessionId": THREAD, "prompt": blocks,
                }),
            ));
            let start = harness(&outs[0]);
            assert_eq!(start["method"], "turn/start");
            let input = start["params"]["input"].as_array().unwrap();
            assert_eq!(input.len(), if caption.is_empty() { 1 } else { 2 });
            if !caption.is_empty() {
                assert_eq!(input[0]["text"], caption);
            }
            let file = input.last().unwrap();
            assert_eq!(file["type"], "text");
            assert_eq!(
                file["text"],
                format!(
                    "Attached file: {}\nLocation: {}\nThe file contents were not embedded. Use available tools to inspect this attachment.",
                    json!(name),
                    json!(path.display().to_string()),
                )
            );
        }
    }
}

#[test]
fn resource_links_preserve_urls_and_quote_untrusted_names() {
    let mut bridge = opened();
    let outs = bridge.on_session(request(2, "session/prompt", json!({
        "sessionId": THREAD,
        "prompt": [{"type": "resource_link", "name": "clip\n\"quoted\"", "uri": "https://example.com/clip.mp4"}],
    })));
    let file = &harness(&outs[0])["params"]["input"][0];
    assert_eq!(file["type"], "text");
    assert_eq!(
        file["text"],
        "Attached file: \"clip\\n\\\"quoted\\\"\"\nLocation: \"https://example.com/clip.mp4\"\nThe file contents were not embedded. Use available tools to inspect this attachment."
    );
}
