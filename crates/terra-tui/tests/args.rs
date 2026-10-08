use std::path::PathBuf;

use terra_tui::args::Args;

fn parse(args: &[&str]) -> Result<Args, String> {
    Args::parse(args.iter().map(|arg| arg.to_string()))
}

#[test]
fn with_no_flags_everything_is_default() {
    let args = parse(&[]).expect("valid");
    assert_eq!(args, Args::default());
    assert_eq!((args.seed, args.preset, args.ascii), (None, None, false));
}

#[test]
fn the_seed_preset_and_ascii_flags_are_read() {
    let args = parse(&["--seed", "42", "--preset", "worlds/big.ron", "--ascii"]).expect("valid");
    assert_eq!(args.seed, Some(42));
    assert_eq!(args.preset, Some(PathBuf::from("worlds/big.ron")));
    assert!(args.ascii);
}

#[test]
fn the_hidden_force_panic_flag_is_read() {
    assert!(parse(&["--force-panic"]).expect("valid").force_panic);
}

#[test]
fn bad_flags_are_errors_that_name_the_problem() {
    let cases: [(&[&str], &str); 5] = [
        (&["--sed", "1"], "--sed"),
        (&["--seed"], "--seed"),
        (&["--seed", "-3"], "-3"),
        (&["--seed", "banana"], "banana"),
        (&["--preset"], "--preset"),
    ];
    for (args, named) in cases {
        let error = parse(args).expect_err("invalid");
        assert!(error.contains(named), "{args:?}: {error:?}");
    }
}

#[test]
fn the_data_and_replay_flags_are_read() {
    let args = parse(&["--data", "mods/bigger", "--seed", "3"]).expect("valid");
    assert_eq!(args.data, Some(PathBuf::from("mods/bigger")));
    let args = parse(&["--replay", "last_session.replay", "--ascii"]).expect("valid");
    assert_eq!(args.replay, Some(PathBuf::from("last_session.replay")));
    assert!(args.ascii);
}

#[test]
fn a_replay_refuses_the_flags_that_make_a_new_world() {
    for flags in [
        &["--replay", "a.replay", "--seed", "1"][..],
        &["--preset", "p.ron", "--replay", "a.replay"],
        &["--replay", "a.replay", "--data", "mods"],
    ] {
        let error = parse(flags).expect_err("refused");
        assert!(error.contains("--replay"), "{flags:?}: {error}");
        assert!(
            error.contains(
                flags
                    .iter()
                    .find(|f| **f != "--replay" && f.starts_with("--"))
                    .unwrap()
            ),
            "{error}"
        );
    }
    for flag in ["--data", "--replay"] {
        assert!(parse(&[flag]).expect_err("needs a value").contains(flag));
    }
}

#[test]
fn the_theme_flag_names_a_theme_file() {
    let args = parse(&["--theme", "themes/night.ron"]).expect("valid");
    assert_eq!(args.theme, Some(PathBuf::from("themes/night.ron")));
    assert_eq!(parse(&[]).expect("valid").theme, None);
}

#[test]
fn the_theme_flag_needs_a_file_and_does_not_mix_with_ascii() {
    let error = parse(&["--theme"]).expect_err("invalid");
    assert!(error.contains("--theme"), "{error}");
    let error = parse(&["--ascii", "--theme", "night.ron"]).expect_err("invalid");
    assert!(
        error.contains("--ascii") && error.contains("--theme"),
        "{error}"
    );
}

#[test]
fn the_version_flag_is_read_and_the_version_names_the_commit() {
    assert!(parse(&["--version"]).expect("valid").version);
    assert!(!parse(&[]).expect("valid").version);
    // "0.1.0 (cc96347)": the package version, then the commit it was built
    // from, so the owner can tell which build they're trying.
    let version = terra_tui::args::VERSION;
    assert!(version.starts_with(env!("CARGO_PKG_VERSION")), "{version}");
    let commit = version
        .split_once(" (")
        .and_then(|(_, rest)| rest.strip_suffix(')'))
        .expect("a commit in brackets");
    assert!(!commit.is_empty(), "{version}");
}
