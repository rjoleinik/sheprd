use crate::{
    browser::TabId,
    chat::tests::core_panes::{DRIVEN, attached_on, id},
    core_terminals::menu::{Action, Target},
    menu::Page,
};
use gpui::{TestAppContext, point, px};
use sheprd_client::ConnectTarget;

#[gpui::test]
fn core_chat_menu_targets_its_pane_on_local_and_remote_hosts(cx: &mut TestAppContext) {
    for target in [
        ConnectTarget::Local,
        ConnectTarget::Ssh {
            target: "host".into(),
            session: "default".into(),
        },
    ] {
        let (view, _stream, _program, cx) = attached_on(cx, target);
        cx.update(|window, cx| {
            view.update(cx, |view, cx| {
                let tab = TabId::test(DRIVEN);
                let terminal = id("t1");
                let key = view.core_chat_key(tab, &terminal);
                // The chat is beside the active shell; no Herdr 1 surface is needed.
                view.live.surface = None;
                assert_eq!(view.core_terminals.chat_split_at(tab), Some(id("t2")));
                view.open_chat_pane_menu(&key, point(px(20.), px(20.)), window, cx);
                assert_eq!(view.menu.page, Some(Page::CoreMenu));
                assert!(view.menu.opening_right_click);
                let expected = Target::Pane { tab, terminal };
                assert_eq!(view.menu.core.as_ref().unwrap().target(), &expected);
                let actions = view.core_menu_actions(&expected, cx);
                for action in [
                    Action::Rename,
                    Action::SplitRight,
                    Action::SplitDown,
                    Action::Zoom,
                    Action::MoveTo,
                    Action::Close,
                ] {
                    assert!(actions.contains(&action));
                }
            });
        });
    }
}
