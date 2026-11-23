#![allow(clippy::unwrap_used)]

use crate::filesystem::rename_noreplace;

#[test]
fn an_existing_file_is_not_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let target = dir.path().join("target");
    std::fs::write(&source, "new").unwrap();
    std::fs::write(&target, "old").unwrap();
    assert!(rename_noreplace(&source, &target).is_err());
    assert_eq!(std::fs::read_to_string(&source).unwrap(), "new");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "old");
}

#[cfg(windows)]
#[test]
fn a_nul_cannot_truncate_either_path() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let target = dir.path().join("target");
    std::fs::create_dir(&source).unwrap();
    for (from, to) in [
        (source.join("\0ignored"), target.clone()),
        (source.clone(), target.join("\0ignored")),
    ] {
        let error = rename_noreplace(&from, &to).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(source.is_dir());
        assert!(!target.exists());
    }
}
