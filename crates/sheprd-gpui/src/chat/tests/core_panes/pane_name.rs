use crate::{
    browser::TabId,
    chat::tests::core_panes::{DRIVEN, attached_on, id},
    core_terminals::menu::Target,
};
use gpui::TestAppContext;
use sheprd_client::ConnectTarget;

#[gpui::test]
fn core_chat_header_tracks_local_and_remote_pane_names(cx: &mut TestAppContext) {
    for target in [
        ConnectTarget::Local,
        ConnectTarget::Ssh {
            target: "host".into(),
            session: "default".into(),
        },
    ] {
        let (view, _stream, _program, cx) = attached_on(cx, target);
        view.update(cx, |view, cx| {
            let tab = TabId::test(DRIVEN);
            let terminal = id("t1");
            let key = view.core_chat_key(tab, &terminal);
            let target = Target::Pane { tab, terminal };
            for (name, expected) in [
                (Some("First task"), "First task"),
                (Some("Second task"), "Second task"),
                (None, "t1"),
            ] {
                view.set_core_name(&target, name.map(str::to_owned), cx);
                let chat = view.chats.pane(&key).unwrap();
                assert_eq!(view.chat_pane_name(chat, cx).as_deref(), Some(expected));
            }
        });
    }
}
