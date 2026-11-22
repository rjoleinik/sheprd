use super::*;
use core::prelude::v1::test;

#[test]
fn repository_identity_survives_worktree_renames() {
    let mut session = crate::session_model::of_snapshot(&crate::sidebar::layout_tests::snapshot(6));
    let parent = &mut session.workspaces[3];
    assert_eq!(name(parent), Some("agent-launcher"));
    parent.label = "renamed main checkout".into();
    assert_eq!(name(parent), Some("agent-launcher"));
    let child = &mut session.workspaces[4];
    child.label = "renamed worktree".into();
    assert_eq!(name(child), Some("agent-launcher"));
    child.branch = None;
    assert_eq!(name(child), None);
}

#[test]
fn custom_spacing_takes_precedence_over_group_defaults() -> Result<()> {
    let mut config = Config::default();
    for mode in [LayoutMode::default(), LayoutMode::Minimal] {
        config.layout.mode = mode;
        config.sidebar_layout.spaces = toml::from_str(
            r#"rows = [["workspace"]]
row_gap = 0"#,
        )?;
        assert_eq!(gap(&config), 0.);
        config.sidebar_layout.spaces.row_gap = 2;
        assert_eq!(gap(&config), 2. * line_height(&config.sidebar));
    }
    Ok(())
}

#[test]
fn initials_are_bounded_and_follow_repository_words() {
    for (name, expected) in [
        ("herdr-gpui", "HG"),
        ("moonmap", "MO"),
        ("flexcolor-rust", "FR"),
        ("  msgvault__moonframe", "MM"),
        ("a", "A"),
        ("---", "?"),
        ("東京-repo", "東"),
        ("école-outils", "ÉO"),
        ("ßeta", "SS"),
    ] {
        assert_eq!(initials(name), expected, "{name}");
    }
}

#[test]
fn letters_contrast_on_every_builtin_theme() {
    for name in Theme::BUILTIN_NAMES {
        let Some(theme) = Theme::builtin(name) else {
            panic!("missing theme {name}");
        };
        for repository in ["herdr-gpui", "moonmap", "moltis", "moonswift"] {
            let tile = Tile::new(repository, &theme);
            assert!(
                contrast::ratio(tile.foreground, tile.background) >= 4.5,
                "{name}: {repository}"
            );
            let same = Tile::new(&repository.to_uppercase(), &theme);
            assert_eq!(tile.background, same.background);
            assert_eq!(tile.letters, same.letters);
        }
    }
}
