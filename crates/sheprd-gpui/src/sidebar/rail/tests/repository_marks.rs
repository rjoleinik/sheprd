use super::*;

#[gpui::test]
fn repository_tiles_keep_indices_and_status_columns(cx: &mut TestAppContext) {
    let (view, cx) = rail_window(cx, 2);
    let mark = placed(cx, "rail-mark-rail-workspace-local-w3");
    let tile = placed(cx, "rail-repository-rail-workspace-local-w3");
    let index = placed(cx, "rail-index-rail-workspace-local-w3");
    let dot = placed(cx, "rail-dot-rail-workspace-local-w3");
    assert!(contains(mark, tile));
    assert!(contains(mark, index));
    assert!(index.right() <= tile.left());
    assert!(tile.right() <= dot.left());
    assert!(
        cx.debug_bounds("rail-repository-rail-workspace-local-w4")
            .is_none()
    );
    assert!(
        cx.debug_bounds("rail-mark-rail-workspace-local-w4")
            .is_some()
    );
    assert!(
        cx.debug_bounds("rail-repository-rail-workspace-ssh:host1-w0")
            .is_some()
    );
    let separator = placed(cx, "repository-separator-rail-workspace-local-w3");
    let previous = placed(cx, "rail-workspace-local-w2");
    let root = placed(cx, "rail-workspace-local-w3");
    assert_eq!(separator.top() - previous.bottom(), px(6.));
    assert_eq!(root.top() - separator.bottom(), px(6.));
    assert!(tile.top() - separator.bottom() >= px(6.));
    assert_eq!(placed(cx, "rail-workspace-local-w4").top(), root.bottom());
    assert!(
        cx.debug_bounds("repository-separator-rail-workspace-local-w4")
            .is_none()
    );

    view.update(cx, |view, cx| {
        view.config.show_repository_icons = false;
        cx.notify();
    });
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    assert!(
        cx.debug_bounds("rail-repository-rail-workspace-local-w3")
            .is_none()
    );
    assert!(
        cx.debug_bounds("rail-index-rail-workspace-local-w3")
            .is_none()
    );
    assert!(
        cx.debug_bounds("rail-mark-rail-workspace-local-w3")
            .is_some()
    );
    assert!(
        cx.debug_bounds("rail-dot-rail-workspace-local-w3")
            .is_some()
    );
}

#[gpui::test]
fn plain_folders_and_standalone_worktrees_keep_numbers(cx: &mut TestAppContext) {
    let (view, cx) = rail_window(cx, 1);
    view.update(cx, |view, cx| {
        let snapshot = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
        snapshot.workspaces[0].branch = None;
        snapshot.workspaces[3].worktree = None;
        cx.notify();
    });
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    for (tile, mark) in [
        (
            "rail-repository-rail-workspace-local-w0",
            "rail-mark-rail-workspace-local-w0",
        ),
        (
            "rail-repository-rail-workspace-local-w4",
            "rail-mark-rail-workspace-local-w4",
        ),
    ] {
        assert!(cx.debug_bounds(tile).is_none());
        assert!(cx.debug_bounds(mark).is_some());
    }
}
