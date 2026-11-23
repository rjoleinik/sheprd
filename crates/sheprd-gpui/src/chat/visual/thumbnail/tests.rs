#![allow(clippy::unwrap_used)]
use crate::chat::visual::{self, thumbnail};
use crate::window::FileLink;
use std::path::Path;

fn file(path: &Path) -> FileLink {
    FileLink {
        path: path.to_str().unwrap().into(),
        line: None,
        pane_id: "preview-test".into(),
        cwd: None,
        explicit: true,
    }
}

#[test]
fn only_pdf_and_video_extensions_request_native_previews() {
    for name in [
        "report.PDF",
        "clip.mp4",
        "a.m4v",
        "a.MOV",
        "a.avi",
        "a.mkv",
        "a.webm",
    ] {
        assert!(thumbnail::supported(Path::new(name)), "{name}");
    }
    for name in [
        "run.sh",
        "app.app",
        "file.pdf.sh",
        "document",
        "a.html",
        "a.zip",
    ] {
        assert!(!thumbnail::supported(Path::new(name)), "{name}");
    }
}

#[test]
fn unavailable_preview_is_a_typed_failure() {
    let directory = tempfile::tempdir().unwrap();
    assert!(matches!(
        thumbnail::draw(&file(&directory.path().join("missing.pdf"))),
        Err(visual::Error::Thumbnail(thumbnail::Error::Unavailable))
    ));
}

#[cfg(target_os = "macos")]
#[test]
fn preview_refuses_directories_executables_large_files_and_network_symlinks() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("directory.pdf");
    std::fs::create_dir(&directory).unwrap();
    let executable = root.path().join("run.pdf");
    std::fs::write(&executable, b"#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let network = root.path().join("network.pdf");
    symlink("/net/example.invalid/report.pdf", &network).unwrap();
    let large = root.path().join("large.pdf");
    std::fs::File::create(&large)
        .unwrap()
        .set_len(1024 * 1024 * 1024 + 1)
        .unwrap();
    for path in [directory, executable, network, large] {
        assert!(file(&path).thumbnail_path().is_none(), "{path:?}");
    }
    let script = root.path().join("script.sh");
    std::fs::write(&script, b"not a PDF").unwrap();
    let disguised = root.path().join("disguised.pdf");
    symlink(&script, &disguised).unwrap();
    assert!(matches!(
        thumbnail::draw(&file(&disguised)),
        Err(visual::Error::Thumbnail(thumbnail::Error::Unavailable))
    ));
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires macOS Quick Look services; run explicitly on a desktop"]
fn native_pdf_and_video_thumbnails_are_real_bounded_images() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/attachments");
    for name in ["Review report.pdf", "Video walkthrough.mp4"] {
        let drawn = thumbnail::draw(&file(&root.join(name))).unwrap();
        assert!(drawn.width > 0. && drawn.width <= 256.);
        assert!(drawn.height > 0. && drawn.height <= 256.);
        assert!(drawn.texture.as_bytes(0).unwrap().len() <= 256 * 256 * 4);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn thumbnail_output_is_bounded_before_decoding_and_preserves_errors() {
    use std::error::Error as _;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("preview.png");
    std::fs::File::create(&path)
        .unwrap()
        .set_len(2 * 1024 * 1024 + 1)
        .unwrap();
    assert!(matches!(
        thumbnail::read(&path),
        Err(thumbnail::Error::Limit)
    ));
    image::RgbaImage::new(257, 1).save(&path).unwrap();
    assert!(matches!(
        thumbnail::read(&path),
        Err(thumbnail::Error::Limit)
    ));
    std::fs::write(&path, b"invalid PNG").unwrap();
    let error = thumbnail::read(&path).unwrap_err();
    assert!(matches!(error, thumbnail::Error::Decode(_)));
    assert!(error.source().is_some());
    std::fs::remove_file(&path).unwrap();
    let error = thumbnail::read(&path).unwrap_err();
    assert!(matches!(error, thumbnail::Error::Io(_)));
    assert!(error.source().is_some());
}

#[cfg(target_os = "macos")]
#[test]
fn timed_out_thumbnail_process_is_killed_and_reaped() {
    let mut child = sheprd_client::new_command("/bin/sleep")
        .arg("30")
        .spawn()
        .unwrap();
    assert!(matches!(
        thumbnail::wait(&mut child, std::time::Duration::ZERO),
        Err(thumbnail::Error::Limit)
    ));
    assert!(child.try_wait().unwrap().is_some());
}

#[cfg(target_os = "macos")]
#[test]
fn file_version_detects_overwrites_replacement_and_disappearance() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("report.pdf");
    let link = file(&path);
    assert!(thumbnail::version(&link).is_none());
    std::fs::write(&path, b"one").unwrap();
    let first = thumbnail::version(&link).unwrap();
    assert!(thumbnail::version(&link).as_ref() == Some(&first));
    std::fs::write(&path, b"longer").unwrap();
    let overwritten = thumbnail::version(&link).unwrap();
    assert!(first != overwritten);
    let replacement = root.path().join("replacement.pdf");
    std::fs::write(&replacement, b"longer").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&replacement)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(overwritten.modified.unwrap()))
        .unwrap();
    std::fs::rename(replacement, &path).unwrap();
    assert!(thumbnail::version(&link).unwrap() != overwritten);
    std::fs::remove_file(path).unwrap();
    assert!(thumbnail::version(&link).is_none());
}

#[gpui::test]
fn visible_thumbnail_checks_are_throttled_and_unchanged_failures_are_reused(
    cx: &mut gpui::TestAppContext,
) {
    use crate::{
        chat::visual::{DiagramStyle, Source, VisualId, Visuals},
        config::{Config, Theme},
    };
    use std::time::{Duration, Instant};
    let renderer = cx.update(|cx| cx.svg_renderer());
    let style = DiagramStyle::of(&Theme::default(), &Config::default().ui);
    // With no cwd this relative path cannot resolve, on any machine.
    let link = file(Path::new("not-written-yet.pdf"));
    let id = VisualId::of("thumbnail-refresh");
    let mut visuals = Visuals::default();
    visuals.request_visible(id, Source::Thumbnail(link));
    let jobs = visuals.jobs_at(2, &style, &renderer, Instant::now());
    assert_eq!(jobs.len(), 1);
    for job in jobs {
        let (id, ticket, image) = job.run();
        assert!(visuals.finish(id, ticket, image));
    }
    assert!(
        visuals
            .jobs_at(2, &style, &renderer, Instant::now())
            .is_empty()
    );
    let later = Instant::now() + Duration::from_secs(3);
    let jobs = visuals.jobs_at(2, &style, &renderer, later);
    assert_eq!(jobs.len(), 1);
    for job in jobs {
        let (id, ticket, image) = job.run();
        assert!(
            !visuals.finish(id, ticket, image),
            "unchanged metadata must not redraw"
        );
    }
    assert!(visuals.take_released().is_empty());
    visuals.begin_frame();
    assert!(
        visuals
            .jobs_at(2, &style, &renderer, later + Duration::from_secs(3))
            .is_empty()
    );
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires macOS Quick Look services; run explicitly on a desktop"]
fn native_revised_pdf_replaces_cached_pixels_and_missing_files_recover() {
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("report.pdf");
    let link = file(&path);
    let missing = thumbnail::refresh(&link, None);
    assert!(missing.image.is_err());
    let original = include_str!("../../../../tests/fixtures/attachments/Review report.pdf");
    std::fs::write(&path, original).unwrap();
    let first = thumbnail::refresh(&link, Some(missing));
    let first_pixels = first
        .image
        .as_ref()
        .as_ref()
        .unwrap()
        .texture
        .as_bytes(0)
        .unwrap()
        .to_vec();
    let unchanged = thumbnail::refresh(&link, Some(first.clone()));
    assert!(Arc::ptr_eq(&first.image, &unchanged.image));
    // Equal-length PDF edit keeps its object offsets valid.
    std::fs::write(&path, original.replace("0.12 0.2 0.45", "0.80 0.2 0.45")).unwrap();
    let revised = thumbnail::refresh(&link, Some(first));
    let revised_pixels = revised
        .image
        .as_ref()
        .as_ref()
        .unwrap()
        .texture
        .as_bytes(0)
        .unwrap();
    assert_ne!(first_pixels, revised_pixels);
    std::fs::remove_file(path).unwrap();
    assert!(thumbnail::refresh(&link, Some(revised)).image.is_err());
}
