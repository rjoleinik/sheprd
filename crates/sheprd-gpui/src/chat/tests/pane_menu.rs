use crate::{
    chat::tests::{
        Program, agent, cursor,
        overlay::{attach_with, open, poll},
        selection::acknowledge_size,
        snapshot,
    },
    menu::Page,
};
use gpui::{Modifiers, MouseButton, TestAppContext};
use sheprd_client::ConnectTarget;

#[gpui::test]
fn chat_header_opens_the_pane_menu_but_transcript_does_not(cx: &mut TestAppContext) {
    let (view, _peer, cx) = open(cx, ConnectTarget::Local);
    let program = Program::new();
    let _stream = attach_with(
        &view,
        &program,
        snapshot(cursor(1, 0), vec![agent(1, "Right click this message.")]),
        cx,
    );
    acknowledge_size(&view, cx);
    poll(&view, &program, cx);
    let prose = cx.debug_bounds("chat-prose").unwrap().center();
    cx.simulate_mouse_down(prose, MouseButton::Right, Modifiers::default());
    cx.simulate_mouse_up(prose, MouseButton::Right, Modifiers::default());
    assert!(view.read_with(cx, |view, _| view.menu.page.is_none()));

    let header = cx.debug_bounds("chat-agent").unwrap().center();
    cx.simulate_mouse_down(header, MouseButton::Right, Modifiers::default());
    poll(&view, &program, cx);
    view.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(Page::Pane));
        assert_eq!(view.pane_menu_target(), Some("w1:p1"));
        assert!(view.menu.opening_right_click);
    });
    cx.simulate_mouse_up(header, MouseButton::Right, Modifiers::default());
    view.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(Page::Pane));
        assert!(!view.menu.opening_right_click);
    });
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.dismiss_menu(window, cx);
            view.live.surface = None;
            view.open_chat_pane_menu("w1:p1", header, window, cx);
            assert!(
                view.menu.page.is_none(),
                "stale surface must not open a menu"
            );
        });
    });
}
