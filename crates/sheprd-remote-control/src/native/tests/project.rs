//! New projects: the checks made before Git runs, an empty repository put
//! in a project root, a clone that fails leaving nothing behind, and a
//! workspace allowed in the new folder.

use crate::native::project::{CreatedProject, Error, check_name, clone_url, last_lines};
use crate::native::tests::*;
use crate::web::tests::{post, send, signed_in};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use serde_json::json;

/// A state whose one workspace sits in `root/app`, so `root` is offered.
fn with_root(dir: &std::path::Path) -> (Shared, String) {
    let root = dir.join("code");
    std::fs::create_dir_all(root.join("app")).unwrap();
    let mut layout = layout(EndpointId::random());
    layout.workspaces[0].cwd = root.join("app").to_str().unwrap().to_owned();
    let state = offline(&dir.join("state"), layout);
    (state, root.to_str().unwrap().to_owned())
}

fn authorized(mut request: Request<Body>, token: &str) -> Request<Body> {
    request.headers_mut().insert(
        header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    );
    request
}

fn error_of(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body).unwrap()["error"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// The entries in `root`, hidden ones included, sorted.
fn entries(root: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[core::prelude::v1::test]
fn names_are_one_visible_segment() {
    assert_eq!(check_name("  weather ").unwrap(), "weather");
    assert_eq!(check_name("my app").unwrap(), "my app");
    for bad in [
        "", " ", ".hidden", "-rf", "a/b", "a\\b", "..", "a:b", "a\nb",
    ] {
        assert!(matches!(check_name(bad), Err(Error::Name)), "{bad:?}");
    }
    assert!(check_name(&"x".repeat(101)).is_err());
}

#[core::prelude::v1::test]
fn clone_addresses_are_github_names_or_remote_urls() {
    assert_eq!(
        clone_url("sheprd-labs/weather").unwrap(),
        "https://github.com/sheprd-labs/weather.git"
    );
    assert_eq!(
        clone_url("owner/.github").unwrap(),
        "https://github.com/owner/.github.git"
    );
    for bad in ["owner/.", "owner/.."] {
        assert!(matches!(clone_url(bad), Err(Error::Url)));
    }
    for kept in [
        "https://github.com/sheprd-labs/weather.git",
        "ssh://git@example.com/team/app.git",
        "git@github.com:sheprd-labs/weather.git",
    ] {
        assert_eq!(clone_url(kept).unwrap(), kept);
    }
    for bad in [
        "",
        "-uhttps://x",
        "/tmp/repo",
        "./repo",
        "file:///tmp/repo",
        "ext::sh -c touch% /tmp/x",
        "https://",
        "owner/",
        "a b/c",
        "git@host:/abs",
        "C:\\repo",
    ] {
        assert!(matches!(clone_url(bad), Err(Error::Url)), "{bad:?}");
    }
}

#[core::prelude::v1::test]
fn git_output_keeps_its_end_on_a_character_boundary() {
    let long = format!("{}é fatal: no", "x".repeat(2000));
    let kept = last_lines(long.as_bytes());
    assert!(kept.len() <= 600);
    assert!(kept.ends_with("fatal: no"));
}

#[tokio::test]
async fn an_empty_project_is_a_git_repository_on_main_in_the_root() {
    let dir = tempfile::tempdir().unwrap();
    let (state, root) = with_root(dir.path());
    let (_, token) = signed_in(&state, 7);
    let request = authorized(
        post("/api/v2/projects", json!({"root": root, "name": "weather"})),
        &token,
    );
    let (status, _, body) = send(&state, request).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let created: CreatedProject = serde_json::from_str(&body).unwrap();
    let path = std::path::Path::new(&root).join("weather");
    assert_eq!(created.path, path.to_str().unwrap());
    let head = std::fs::read_to_string(path.join(".git/HEAD")).unwrap();
    assert_eq!(head.trim(), "ref: refs/heads/main");
    // Git's scratch folder is gone.
    assert_eq!(entries(&root), ["app", "weather"]);

    // Asking again finds it there and changes nothing.
    let request = authorized(
        post("/api/v2/projects", json!({"root": root, "name": "weather"})),
        &token,
    );
    let (status, _, body) = send(&state, request).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(error_of(&body), "weather already exists there");
}

#[tokio::test]
async fn a_new_folder_past_the_pickers_limit_still_takes_a_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let (state, root) = with_root(dir.path());
    // More folders than the picker lists, sorted before the new one.
    for index in 0..70 {
        std::fs::create_dir(std::path::Path::new(&root).join(format!("a{index:02}"))).unwrap();
    }
    std::fs::create_dir(std::path::Path::new(&root).join("zz-new")).unwrap();
    let offered = crate::web::folders(&state.hub.layout());
    let new = format!("{root}/zz-new");
    assert!(offered.roots[0].1.iter().all(|folder| *folder != new));
    assert!(offered.offers(&new));
    assert!(!offered.offers(&format!("{root}/missing")));
    assert!(!offered.offers(&format!("{root}/zz-new/inner")));
    assert!(!offered.offers(&format!("{root}/..")));
}

#[tokio::test]
async fn a_request_is_checked_before_git_runs() {
    let dir = tempfile::tempdir().unwrap();
    let (state, root) = with_root(dir.path());
    let (_, token) = signed_in(&state, 7);
    let elsewhere = dir.path().to_str().unwrap().to_owned();
    for (body, error) in [
        (
            json!({"root": elsewhere, "name": "x"}),
            "that is not a folder new projects go in",
        ),
        (
            json!({"root": root, "name": "../x"}),
            "that is not a folder name a project can have",
        ),
        (
            json!({"root": root, "name": "x", "clone": "/etc"}),
            "that is not a repository address this computer clones",
        ),
    ] {
        let request = authorized(post("/api/v2/projects", body), &token);
        let (status, _, answer) = send(&state, request).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
        assert_eq!(error_of(&answer), error);
    }
    assert_eq!(entries(&root), ["app"]);
    let (status, _, _) = send(
        &state,
        post("/api/v2/projects", json!({"root": root, "name": "x"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_clone_git_refuses_leaves_nothing_behind() {
    let dir = tempfile::tempdir().unwrap();
    let (state, root) = with_root(dir.path());
    let (_, token) = signed_in(&state, 7);
    // Nothing listens on port 1, so Git fails at once.
    let request = authorized(
        post(
            "/api/v2/projects",
            json!({"root": root, "name": "weather", "clone": "https://127.0.0.1:1/x.git"}),
        ),
        &token,
    );
    let (status, _, body) = send(&state, request).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert!(error_of(&body).starts_with("git refused: "), "{body}");
    assert_eq!(entries(&root), ["app"]);
}

#[cfg(unix)]
#[core::prelude::v1::test]
fn an_unlisted_symlink_cannot_escape_the_project_root() {
    let dir = tempfile::tempdir().unwrap();
    let (state, root) = with_root(dir.path());
    let outside = dir.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    let link = std::path::Path::new(&root).join("linked");
    std::os::unix::fs::symlink(&outside, &link).unwrap();
    let offered = crate::web::folders(&state.hub.layout());
    assert!(!offered.offers(link.to_str().unwrap()));
}
