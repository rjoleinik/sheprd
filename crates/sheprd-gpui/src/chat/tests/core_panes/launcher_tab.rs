use super::*;

/// The tab strip's agent button opens a chat in a tab of its own beside a
/// herdr-core tab, as beside a Herdr 1 tab, rather than splitting the pane
/// in use. A tab the window does not list has no workspace to open it in.
#[gpui::test]
fn the_agent_button_opens_a_tab_not_a_split_beside_a_core_tab(cx: &mut TestAppContext) {
    let (view, _peer, _program, cx) = attached(cx);
    view.update(cx, |view, cx| {
        let tab = TabId::test(DRIVEN);
        let panes = |view: &SheprdWindow| -> Vec<TerminalId> {
            view.core_terminals
                .visible_chat_panes(tab)
                .into_iter()
                .cloned()
                .collect()
        };
        let before = panes(view);
        assert!(matches!(
            view.start_core_chat_tab_beside(sheprd_chat_protocol::Agent::Codex, tab, cx),
            Err(StartError::NoWorkspace)
        ));
        // Nothing was split into the tab meanwhile.
        assert_eq!(panes(view), before);
    });
}
