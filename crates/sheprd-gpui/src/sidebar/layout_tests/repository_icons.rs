use super::*;

#[gpui::test]
fn turning_the_repository_icons_off_gives_the_name_their_room(cx: &mut gpui::TestAppContext) {
    let (fixture, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| fixture_window(window, cx));
        cx.observe(&view, |_, _, cx| cx.notify()).detach();
        SidebarFixture(view)
    });
    let view = cx.update(|_, cx| fixture.read(cx).0.clone());
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.run_until_parked();
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    let icon = cx.debug_bounds("github-herdr").unwrap();
    let tile = cx.debug_bounds("repository-tile-HE").unwrap();
    assert_eq!(tile, icon);
    assert_eq!(tile.size, size(px(16.), px(16.)));
    let shown = cx.debug_bounds("name-herdr").unwrap();
    assert!(cx.debug_bounds("github-agent-launcher").is_some());

    view.update(cx, |view, cx| {
        view.config.show_repository_icons = false;
        cx.notify();
    });
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("github-herdr").is_none());
    assert!(cx.debug_bounds("github-agent-launcher").is_none());
    // The name starts where the icon did and keeps its right edge.
    let hidden = cx.debug_bounds("name-herdr").unwrap();
    assert_eq!(hidden.left(), icon.left());
    assert_eq!(hidden.right(), shown.right());
    assert_eq!(
        hidden.size.width,
        shown.size.width + px(crate::sidebar::ICON_RESERVE)
    );

    view.update(cx, |view, cx| {
        view.config.show_repository_icons = true;
        cx.notify();
    });
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    assert_eq!(cx.debug_bounds("github-herdr"), Some(icon));
    assert_eq!(cx.debug_bounds("name-herdr"), Some(shown));
}

/// Superset draws an icon slot of its own, which the setting leaves in place:
/// only what is drawn inside it changes.
#[gpui::test]
fn superset_keeps_its_icon_slot_without_the_repository_icons(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(800.), px(600.)));
    view.update(cx, |view, cx| {
        view.config.layout.mode = crate::config::LayoutMode::Superset;
        cx.notify();
    });
    cx.run_until_parked();
    let slot = cx.debug_bounds("icon-herdr").unwrap();
    let tile = cx.debug_bounds("repository-tile-HE").unwrap();
    assert!(slot.contains(&tile.center()));
    let row = cx.debug_bounds("row-herdr").unwrap();

    view.update(cx, |view, cx| {
        view.config.show_repository_icons = false;
        cx.notify();
    });
    cx.run_until_parked();
    assert_eq!(cx.debug_bounds("icon-herdr"), Some(slot));
    assert_eq!(cx.debug_bounds("row-herdr"), Some(row));
}

#[gpui::test]
fn repository_gaps_separate_roots_but_keep_children_attached(cx: &mut gpui::TestAppContext) {
    for mode in crate::config::LayoutMode::ALL.into_iter().filter(|mode| {
        matches!(
            mode,
            crate::config::LayoutMode::Classic { .. } | crate::config::LayoutMode::Minimal
        )
    }) {
        let (view, cx) = cx.add_window_view(|window, cx| {
            let mut view = fixture_window(window, cx);
            view.config.layout.mode = mode;
            view.sidebar_split = Some(0.9);
            view
        });
        cx.simulate_resize(size(px(800.), px(900.)));
        cx.update(|window, cx| full_draw(window, cx).clear(cx));
        let first = cx.debug_bounds("row-herdr").unwrap();
        let second = cx
            .debug_bounds("row-herdr-gpui-sidebar-rendering-regression-investigation")
            .unwrap();
        let gap = view.read_with(cx, |view, _| {
            super::super::scaled(
                &view.config.sidebar,
                if mode == crate::config::LayoutMode::Minimal {
                    16.
                } else {
                    7.
                },
            )
        });
        assert_eq!(second.top() - first.bottom(), px(gap), "{mode}");
        if mode == crate::config::LayoutMode::Minimal {
            let rule = cx
                .debug_bounds(
                    "repository-separator-herdr-gpui-sidebar-rendering-regression-investigation",
                )
                .unwrap();
            assert!(rule.top() > first.bottom() && rule.bottom() < second.top());
            assert!(cx.debug_bounds("repository-separator-herdr").is_none());
            assert!(
                cx.debug_bounds("repository-separator-sidebar-child")
                    .is_none()
            );
        }
        let parent = cx.debug_bounds("row-agent-launcher").unwrap();
        let child = cx.debug_bounds("row-sidebar-child").unwrap();
        assert_eq!(child.top(), parent.bottom(), "{mode}");

        view.update(cx, |view, cx| {
            view.config.sidebar_layout.spaces = toml::from_str(
                r#"rows = [["workspace"]]
row_gap = 0"#,
            )
            .unwrap();
            cx.notify();
        });
        cx.update(|window, cx| full_draw(window, cx).clear(cx));
        let first = cx.debug_bounds("row-herdr").unwrap();
        let second = cx
            .debug_bounds("row-herdr-gpui-sidebar-rendering-regression-investigation")
            .unwrap();
        assert_eq!(second.top(), first.bottom(), "{mode}");
        assert!(
            cx.debug_bounds(
                "repository-separator-herdr-gpui-sidebar-rendering-regression-investigation"
            )
            .is_none()
        );
    }
}

#[gpui::test]
fn orca_chips_fit_existing_lines_and_follow_the_icon_setting(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.config.layout.mode = crate::config::LayoutMode::Orca;
        view.sidebar_split = Some(0.9);
        view.sidebar_width = Some(180.);
        Arc::make_mut(view.live.snapshot.as_mut().unwrap()).workspaces[4].branch =
            Some("sidebar-child".into());
        view
    });
    cx.simulate_resize(size(px(800.), px(900.)));
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    let parent = cx.debug_bounds("row-agent-launcher").unwrap();
    let child = cx.debug_bounds("row-sidebar-child").unwrap();
    let parent_chip = cx.debug_bounds("repository-chip-agent-launcher").unwrap();
    let child_chip = cx.debug_bounds("repository-chip-sidebar-child").unwrap();
    let parent_name = cx.debug_bounds("name-agent-launcher").unwrap();
    let child_name = cx.debug_bounds("name-sidebar-child").unwrap();
    assert!(parent_chip.top() >= parent_name.bottom());
    assert!(child_name.right() <= child_chip.left());
    assert!(child_chip.right() <= child.right());
    assert!(parent_chip.right() <= parent.right());
    assert_eq!(child_chip.center().y, child_name.center().y);

    view.update(cx, |view, cx| {
        view.config.show_repository_icons = false;
        cx.notify();
    });
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("repository-chip-agent-launcher").is_none());
    assert!(cx.debug_bounds("repository-chip-sidebar-child").is_none());
    assert_eq!(
        cx.debug_bounds("row-agent-launcher").unwrap().size.height,
        parent.size.height
    );
    assert_eq!(
        cx.debug_bounds("row-sidebar-child").unwrap().size.height,
        child.size.height
    );

    view.update(cx, |view, cx| {
        view.config.show_repository_icons = true;
        view.config.sidebar_layout.spaces = toml::from_str(r#"rows = [["workspace"]]"#).unwrap();
        cx.notify();
    });
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("repository-chip-agent-launcher").is_none());
    assert!(cx.debug_bounds("repository-chip-sidebar-child").is_none());
}

/// Configured rows place the icon themselves, beside their first line.
#[gpui::test]
fn configured_rows_give_the_icon_room_to_their_first_line(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(800.), px(600.)));
    view.update(cx, |view, cx| {
        view.config.sidebar_layout.spaces =
            toml::from_str(r#"rows = [["workspace", "branch"]]"#).unwrap();
        cx.notify();
    });
    cx.run_until_parked();
    // One configured line, so the built-in branch line is gone.
    assert!(cx.debug_bounds("detail-herdr").is_none());
    let icon = cx.debug_bounds("github-herdr").unwrap();
    let shown = cx.debug_bounds("name-herdr").unwrap();
    assert_eq!(shown.left(), icon.left() + px(crate::sidebar::ICON_RESERVE));

    view.update(cx, |view, cx| {
        view.config.show_repository_icons = false;
        cx.notify();
    });
    cx.run_until_parked();
    assert!(cx.debug_bounds("github-herdr").is_none());
    let hidden = cx.debug_bounds("name-herdr").unwrap();
    assert_eq!(hidden.left(), icon.left());
    assert_eq!(hidden.right(), shown.right());
}
