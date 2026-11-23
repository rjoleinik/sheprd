use crate::chat::{
    prepared::{Body, Part, Prepared},
    tests::{Program, agent, cursor, overlay, snapshot},
    visual::{self, DiagramStyle, Shown, Source, VisualId, Visuals},
};
use crate::{
    config::{Config, Theme},
    window::FileLink,
};
use gpui::TestAppContext;
use sheprd_chat_protocol::{EntryId, Transcript};
use sheprd_client::ConnectTarget;

fn file() -> FileLink {
    FileLink {
        path: "/missing-thumbnail.pdf".into(),
        line: None,
        pane_id: "w1:p1".into(),
        cwd: None,
        explicit: true,
    }
}

#[gpui::test]
fn thumbnails_share_worker_and_retention_limits_and_cache_failures(cx: &mut TestAppContext) {
    let renderer = cx.update(|cx| cx.svg_renderer());
    let style = DiagramStyle::of(&Theme::default(), &Config::default().ui);
    let mut visuals = Visuals::default();
    for n in 0..visual::MAX_RICH_VISUALS + 10 {
        visuals.want(VisualId::of(n), Source::Thumbnail(file()));
    }
    assert_eq!(visuals.rich_budget().0, visual::MAX_RICH_VISUALS);
    let jobs = visuals.jobs(&style, &renderer);
    assert_eq!(jobs.len(), visual::MAX_RUNNING);
    assert!(visuals.jobs(&style, &renderer).is_empty());
    let mut completed = Vec::new();
    for job in jobs {
        let (id, ticket, result) = job.run();
        visuals.finish(id, ticket, result);
        assert!(matches!(visuals.shown(id), Shown::Failed(_)));
        completed.push(id);
    }
    let next = visuals.jobs(&style, &renderer);
    assert!(next.iter().all(|job| !completed.contains(&job.id)));
}

#[gpui::test]
fn thumbnail_survives_transcript_refresh_and_leaves_with_its_card(cx: &mut TestAppContext) {
    let renderer = cx.update(|cx| cx.svg_renderer());
    let style = DiagramStyle::of(&Theme::default(), &Config::default().ui);
    let mut transcript = Transcript::new("Codex");
    transcript.entries = vec![agent(1, "[report](/missing-thumbnail.pdf)")];
    let mut prepared = Prepared::default();
    prepared.update(&transcript, &[(EntryId(1), 1)]);
    let body = prepared.body(EntryId(1)).unwrap();
    let Body::Message(parts) = body.as_ref() else {
        panic!("message expected");
    };
    let Part::File { id, .. } = &parts[0] else {
        panic!("file card expected");
    };
    let id = *id;
    prepared
        .visuals
        .request_visible(id, Source::Thumbnail(file()));
    for job in prepared.visuals.jobs(&style, &renderer) {
        let (id, ticket, result) = job.run();
        prepared.visuals.finish(id, ticket, result);
    }
    prepared.update(&transcript, &[(EntryId(1), 1)]);
    assert!(matches!(prepared.visuals.shown(id), Shown::Failed(_)));
    assert_eq!(prepared.drawing(id).collect::<Vec<_>>(), [EntryId(1)]);
    assert!(prepared.visuals.jobs(&style, &renderer).is_empty());
    transcript.entries.clear();
    prepared.update(&transcript, &[]);
    assert!(!prepared.visuals.wants(id));
    assert_eq!(prepared.visuals.rich_budget().0, 0);
}

#[gpui::test]
fn remote_cards_never_request_a_local_thumbnail(cx: &mut TestAppContext) {
    let (view, _peer, cx) = overlay::open(
        cx,
        ConnectTarget::Ssh {
            target: "synthetic-host".into(),
            session: "default".into(),
        },
    );
    let program = Program::new();
    let _stream = overlay::attach_with(
        &view,
        &program,
        snapshot(cursor(1, 0), vec![agent(1, "[report](/tmp/report.pdf)")]),
        cx,
    );
    view.update(cx, |view, _| {
        assert!(
            view.chat_file_thumbnail("w1:p1", "/tmp/report.pdf")
                .is_none()
        );
        let pane = view.chats.pane("w1:p1").unwrap();
        assert_eq!(pane.prepared.visuals.rich_budget().0, 0);
    });
}

#[gpui::test]
fn thumbnail_admission_checks_local_session_and_file_type(cx: &mut TestAppContext) {
    let (view, _peer, cx) = overlay::open(cx, ConnectTarget::Local);
    let program = Program::new();
    let _stream = overlay::attach_with(
        &view,
        &program,
        snapshot(cursor(1, 0), vec![agent(1, "Ready")]),
        cx,
    );
    view.update(cx, |view, _| {
        assert_eq!(
            view.chat_file_thumbnail("w1:p1", "/tmp/report.pdf")
                .is_some(),
            cfg!(target_os = "macos")
        );
        for destination in [
            "/tmp/run.sh",
            "https://example.com/movie.mp4",
            "file://remote/report.pdf",
            "/tmp/report.pdf:3",
        ] {
            assert!(view.chat_file_thumbnail("w1:p1", destination).is_none());
        }
        assert!(
            view.chat_file_thumbnail("missing-pane", "/tmp/report.pdf")
                .is_none()
        );
    });
}

#[gpui::test]
fn file_preview_click_uses_the_same_guarded_system_opener(cx: &mut TestAppContext) {
    use crate::chat::tests::{file_links::document_link, wait_until};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("report.pdf");
    std::fs::write(&path, b"unavailable thumbnail").unwrap();
    let (view, _peer, cx) = overlay::open(cx, ConnectTarget::Local);
    view.update(cx, |view, _| {
        view.config.open_files_in = crate::config::FileTarget::Editor
    });
    let program = Program::new();
    let _stream = overlay::attach_with(
        &view,
        &program,
        snapshot(
            cursor(1, 0),
            vec![agent(1, &document_link("report.pdf", &path))],
        ),
        cx,
    );
    assert!(cx.opened_url().is_none());
    let preview = cx.debug_bounds("chat-file-preview").unwrap();
    cx.simulate_click(preview.center(), gpui::Modifiers::default());
    wait_until(|| {
        cx.run_until_parked();
        cx.opened_url().is_some()
    });
    let opened = url::Url::parse(&cx.opened_url().unwrap())
        .unwrap()
        .to_file_path()
        .unwrap();
    assert_eq!(
        std::fs::canonicalize(opened).unwrap(),
        std::fs::canonicalize(path).unwrap()
    );
}
