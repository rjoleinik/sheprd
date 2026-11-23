use crate::machine::{
    Name,
    store::{require_installed_name_in, resolve_name_in, tests::installed},
};

#[test]
fn reservations_use_the_installed_directory_spelling() {
    let root = tempfile::tempdir().unwrap();
    installed(root.path(), "one");
    let name = Name::try_from("one".to_owned()).unwrap();
    let upper = Name::try_from("ONE".to_owned()).unwrap();
    assert_eq!(resolve_name_in(root.path(), &name).unwrap(), name);
    require_installed_name_in(root.path(), &name).unwrap();
    if root.path().join("ONE").exists() {
        // The alternate spelling reaches the same real folder on default macOS volumes.
        assert_eq!(resolve_name_in(root.path(), &upper).unwrap(), name);
        assert!(
            matches!(require_installed_name_in(root.path(), &upper), Err(crate::machine::Error::NameMismatch { requested, installed }) if requested == upper && installed == name)
        );
    } else {
        // A case-sensitive volume can hold two genuinely separate machines.
        installed(root.path(), "ONE");
        assert_eq!(resolve_name_in(root.path(), &upper).unwrap(), upper);
        require_installed_name_in(root.path(), &upper).unwrap();
        assert_eq!(resolve_name_in(root.path(), &name).unwrap(), name);
    }
}

#[test]
fn an_empty_directory_cannot_be_resolved_as_an_installed_machine() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("empty")).unwrap();
    let name = Name::try_from("empty".to_owned()).unwrap();
    assert!(matches!(
        resolve_name_in(root.path(), &name),
        Err(crate::machine::Error::NotInstalled(_))
    ));
}
