//! The cards atop Settings > Fleet that say what can happen next: on a
//! computer in no fleet, each fleet a computer it reaches over SSH is a
//! member of, to join; on a member, the computers it reaches that are in
//! no fleet, to invite all at once. The button says what it does and the
//! card what follows, so the click is the confirmation.

use super::*;
use crate::fleet::{Machine, join};

impl SettingsWindow {
    /// A card for each fleet this computer, in none, may join: Join
    /// through the member `sheprd fleet join` named, else the first, and
    /// Through each other member of that fleet, should one not answer.
    /// Every button waits while a join or an invitation runs.
    pub(super) fn fleet_join_cards(&self, cx: &mut Context<Self>) -> Vec<Div> {
        let theme = &self.theme;
        let busy = Fleet::busy(cx);
        let reading = !Fleet::read(cx);
        Fleet::offers(cx)
            .into_iter()
            .enumerate()
            .map(|(index, offered)| {
                let offer = offered.offer.clone();
                let through = offered.through_first(self.fleet.join_through.as_deref());
                let first = crate::notifications::safe_text(&through[0].1.label, 80);
                let joining = through.iter().any(|(id, _)| Fleet::acting(cx, id));
                let mut actions = Vec::new();
                for (slot, (id, machine)) in through.into_iter().enumerate() {
                    let name = crate::notifications::safe_text(&machine.label, 80);
                    let label = match slot {
                        _ if joining && Fleet::acting(cx, &id) => {
                            fl!("settings-fleet-machine-working")
                        }
                        0 => fl!("settings-fleet-join-card-join", machine = name),
                        _ => fl!("settings-fleet-join-card-through", machine = name),
                    };
                    let id_selector = format!("fleet-join-card-{index}-join-{slot}");
                    let button = if slot == 0 {
                        primary(theme, id_selector, label, !busy)
                    } else {
                        ui::button(theme, id_selector, label, !busy)
                    };
                    actions.push(button.when(!busy, |button| {
                        button.on_click(cx.listener(move |this, _, _, cx| {
                            this.fleet_join(id.clone(), machine.clone(), cx);
                        }))
                    }));
                }
                let can_create = !reading && !busy;
                actions.push(
                    ui::button(
                        theme,
                        format!("fleet-join-card-{index}-create"),
                        fl!("settings-fleet-join-card-create"),
                        can_create,
                    )
                    .when(can_create, |button| {
                        button
                            .on_click(cx.listener(|this, _, _, cx| this.fleet_act(Ask::Found, cx)))
                    }),
                );
                card(
                    theme,
                    format!("fleet-join-card-{index}"),
                    fl!("settings-fleet-join-card", machine = first.clone()),
                    fl!(
                        "settings-fleet-join-card-description",
                        machine = first,
                        members = offer.members,
                        hosts = offer.hosts
                    ),
                    actions,
                )
            })
            .collect()
    }

    /// Joins the fleet `machine` is a member of, and shows a failure.
    fn fleet_join(&mut self, id: String, machine: Machine, cx: &mut Context<Self>) {
        self.fleet.error = None;
        let view = cx.entity().downgrade();
        join::join(cx, id, machine, move |failure, cx| {
            let _ = view.update(cx, |view, cx| {
                view.fleet.error = failure;
                cx.notify();
            });
        });
        cx.notify();
    }

    /// The computers this member reaches that are in no fleet, with Invite
    /// all; none once the person said Not now on this visit.
    pub(super) fn fleet_invite_card(&self, cx: &mut Context<Self>) -> Option<Div> {
        if self.fleet.not_now {
            return None;
        }
        let theme = &self.theme;
        let outside: Vec<(String, Machine)> = Fleet::machines(cx)
            .into_iter()
            .filter(|(id, _)| Fleet::standing(cx, id) == Standing::Outside)
            .collect();
        if outside.is_empty() {
            return None;
        }
        let inviting = outside.iter().any(|(id, _)| Fleet::acting(cx, id));
        let busy = Fleet::busy(cx);
        let count = outside.len();
        let names = outside
            .iter()
            .map(|(_, machine)| crate::notifications::safe_text(&machine.label, 80))
            .collect::<Vec<_>>()
            .join(", ");
        let invite = primary(
            theme,
            "fleet-invite-card-invite",
            if inviting {
                fl!("settings-fleet-machine-working")
            } else {
                fl!("settings-fleet-invite-card-invite", count = count)
            },
            !busy,
        )
        .when(!busy, |button| {
            button.on_click(cx.listener(move |this, _, _, cx| {
                this.fleet.error = None;
                this.fleet.inviting = None;
                let view = cx.entity().downgrade();
                flows::invite_all(cx, outside.clone(), move |failure, cx| {
                    let _ = view.update(cx, |view, cx| {
                        view.fleet.error = failure;
                        cx.notify();
                    });
                });
                cx.notify();
            }))
        });
        let later = ui::button(
            theme,
            "fleet-invite-card-later",
            fl!("settings-fleet-invite-card-later"),
            !inviting,
        )
        .when(!inviting, |button| {
            button.on_click(cx.listener(|this, _, _, cx| {
                this.fleet.not_now = true;
                cx.notify();
            }))
        });
        Some(card(
            theme,
            "fleet-invite-card".into(),
            fl!("settings-fleet-invite-card", count = count),
            fl!(
                "settings-fleet-invite-card-description",
                count = count,
                machines = names
            ),
            vec![invite, later],
        ))
    }
}

/// A button filled with the theme's primary color: the card's main
/// action. Built as [`ui::button`] is, with its own fill and hover.
fn primary(
    theme: &crate::config::Theme,
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    enabled: bool,
) -> Stateful<Div> {
    let (id, label) = (id.into(), label.into());
    let fill = theme.primary();
    let hover = crate::config::mix(fill, theme.background, 15);
    let button = div()
        .id(ElementId::Name(id.clone()))
        .debug_selector(move || id.to_string())
        .role(Role::Button)
        .aria_label(label.clone());
    crate::accessibility::disabled(button, !enabled)
        .flex_none()
        .whitespace_nowrap()
        .px(px(10.))
        .py(px(3.))
        .rounded(px(crate::config::corners::CONTROL))
        .border_1()
        .border_color(rgb(fill))
        .bg(rgb(fill))
        .text_color(rgb(theme.text_on(fill)))
        .when(enabled, |button| {
            button
                .cursor_pointer()
                .hover(move |style| style.bg(rgb(hover)))
        })
        .when(!enabled, |button| button.opacity(0.5))
        .child(label)
}

/// A tinted card: a title, what follows, and the actions.
fn card(
    theme: &crate::config::Theme,
    id: String,
    title: String,
    description: String,
    actions: Vec<Stateful<Div>>,
) -> Div {
    let (title_id, desc_id) = (format!("{id}-title"), format!("{id}-description"));
    div()
        .debug_selector(move || id.clone())
        .flex()
        .flex_col()
        .gap(px(8.))
        .my(px(8.))
        .p(px(12.))
        .rounded(px(crate::config::corners::PANEL))
        .bg(rgb(theme.primary_wash()))
        .child(
            div()
                .debug_selector(move || title_id.clone())
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .debug_selector(move || desc_id.clone())
                .text_size(px(11.))
                .line_height(px(15.))
                .text_color(rgb(theme.subtext()))
                .child(description),
        )
        .child(div().flex().flex_wrap().gap(px(8.)).children(actions))
}
