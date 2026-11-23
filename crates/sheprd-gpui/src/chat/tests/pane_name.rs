use crate::chat::tests::{
    Program, agent, cursor,
    overlay::{attach_with, open, poll},
    snapshot,
};
use gpui::TestAppContext;
use sheprd_client::{ConnectTarget, protocol::ClientShellPane};
use std::sync::Arc;

#[gpui::test]
fn chat_header_tracks_pane_renames_and_clearing(cx: &mut TestAppContext) {
    let (view, _peer, cx) = open(cx, ConnectTarget::Local);
    let program = Program::new();
    let _stream = attach_with(
        &view,
        &program,
        snapshot(cursor(1, 0), vec![agent(1, "Ready.")]),
        cx,
    );
    for (label, expected) in [
        (Some("First task"), Some("First task")),
        (Some("  Second task\u{202e}  "), Some("Second task")),
        (Some("  "), None),
        (None, None),
    ] {
        view.update(cx, |view, cx| {
            let snapshot = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
            let agent = &mut snapshot.agents[0];
            agent.title = None;
            snapshot.panes = vec![ClientShellPane {
                pane_id: agent.pane_id.clone(),
                workspace_id: agent.workspace_id.clone(),
                tab_id: agent.tab_id.clone(),
                label: label.map(str::to_owned),
                cwd: None,
                foreground_cwd: None,
                focused: true,
                right_click_passthrough: false,
            }];
            let chat = view.chats.panes().next().unwrap();
            assert_eq!(view.chat_pane_name(chat, cx).as_deref(), expected);
            cx.notify();
        });
        poll(&view, &program, cx);
        assert_eq!(
            cx.debug_bounds("chat-pane-name").is_some(),
            expected.is_some()
        );
        assert!(cx.debug_bounds("chat-agent").is_some());
    }
}
