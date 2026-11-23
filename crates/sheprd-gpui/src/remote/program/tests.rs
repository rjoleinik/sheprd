#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

const PRINT: &str = concat!(find_program!(), "printf '%s' \"$program\"\n");

/// Makes an executable at `home`/`relative`.
fn program(home: &Path, relative: &str) {
    let path = home.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "#!/bin/sh\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// What the snippet picks, run as SSH would with a short `PATH`.
fn found(home: &Path, path: &str) -> String {
    let output = Command::new("/bin/sh")
        .args(["-c", PRINT])
        .env_clear()
        .env("PATH", path)
        .env("HOME", home)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn a_mac_with_sheprd_only_in_its_app_bundle_is_found() {
    let home = tempfile::tempdir().unwrap();
    let bundled = "Applications/Sheprd.app/Contents/MacOS/sheprd-remote-control";
    program(home.path(), bundled);
    assert_eq!(
        found(home.path(), "/usr/bin:/bin"),
        home.path().join(bundled).to_str().unwrap()
    );
}

#[test]
fn path_then_the_installed_copy_come_before_a_bundle() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    program(
        home,
        "Applications/Sheprd.app/Contents/MacOS/sheprd-remote-control",
    );
    program(home, ".local/bin/sheprd-remote-control");
    let installed = home.join(".local/bin/sheprd-remote-control");
    assert_eq!(found(home, "/usr/bin:/bin"), installed.to_str().unwrap());
    program(home, "bin/herdr-remote-control");
    let on_path = format!("{}:/usr/bin:/bin", home.join("bin").display());
    let first = home.join("bin/herdr-remote-control");
    assert_eq!(found(home, &on_path), first.to_str().unwrap());
}
