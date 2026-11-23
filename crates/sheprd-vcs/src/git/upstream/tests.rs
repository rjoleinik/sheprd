use std::{
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}

#[test]
fn counts_divergence_and_absent_upstream() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();
    let read = || {
        crate::git::upstream::read(
            path.to_str().unwrap(),
            Instant::now() + Duration::from_secs(10),
            &|| false,
        )
        .unwrap()
    };
    git(path, &["init", "-q", "-b", "main"]);
    assert_eq!(read(), None);
    git(path, &["commit", "-q", "--allow-empty", "-m", "base"]);
    git(path, &["branch", "upstream"]);
    git(path, &["branch", "--set-upstream-to=upstream"]);
    assert_eq!(read(), Some((0, 0)));
    git(path, &["commit", "-q", "--allow-empty", "-m", "local"]);
    assert_eq!(read(), Some((1, 0)));
    git(path, &["checkout", "-q", "upstream"]);
    git(path, &["commit", "-q", "--allow-empty", "-m", "remote"]);
    git(path, &["checkout", "-q", "main"]);
    assert_eq!(read(), Some((1, 1)));
    git(path, &["checkout", "-q", "--detach"]);
    assert_eq!(read(), None);
}
