//! Chat headers use the underlying pane's menu, including its targeting
//! and availability rules. Transcript and composer clicks stay in the chat.

use crate::{SheprdWindow, chat::Surface, core_terminals::menu::Target, menu::Page};
use gpui::{Context, Pixels, Point, Window};

impl SheprdWindow {
    pub(in crate::chat) fn open_chat_pane_menu(
        &mut self,
        pane: &str,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.menu.page.is_some() {
            return;
        }
        let Some(chat) = self.chats.pane(pane) else {
            return;
        };
        let page = match &chat.target.surface {
            Surface::Herdr => {
                if !self.input_ready() {
                    return;
                }
                self.open_pane_menu(pane, position, window, cx);
                Page::Pane
            }
            Surface::Core(terminal) | Surface::RemoteCore { terminal, .. } => {
                let Some(tab) = self.core_chat_tab(pane) else {
                    return;
                };
                let target = Target::Pane {
                    tab,
                    terminal: terminal.clone(),
                };
                self.open_core_menu(target, position, window, cx);
                Page::CoreMenu
            }
        };
        self.menu.opening_right_click = self.menu.page == Some(page);
    }
}
