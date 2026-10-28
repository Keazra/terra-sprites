use std::path::PathBuf;

use terra_sim::Mode;
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
fn the_theme_flag_needs_a_file_and_goes_with_ascii() {
    let error = parse(&["--theme"]).expect_err("invalid");
    assert!(error.contains("--theme"), "{error}");
    // `--ascii` picks only the glyph set, so it draws the file's palette in
    // ASCII (design §6.2, #307).
    let args = parse(&["--ascii", "--theme", "night.ron"]).expect("valid");
    assert!(args.ascii);
    assert_eq!(args.theme, Some(PathBuf::from("night.ron")));
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

#[test]
fn the_lab_load_fast_forward_and_save_flags_are_read() {
    // M2 design §16.
    assert_eq!(
        parse(&["--lab", "--seed", "7"]).expect("valid").mode,
        Mode::Lab
    );
    assert_eq!(parse(&["--seed", "7"]).expect("valid").mode, Mode::Story);
    let args = parse(&["--load", "mira"]).expect("valid");
    assert_eq!(args.load.as_deref(), Some("mira"));
    let args =
        parse(&["--fast-forward", "200", "--seed", "7", "--save", "evolved"]).expect("valid");
    assert_eq!(args.fast_forward, Some(200));
    assert_eq!(args.save.as_deref(), Some("evolved"));
    let args = parse(&["--fast-forward", "50", "--load", "mira"]).expect("valid");
    assert_eq!(
        (args.fast_forward, args.load.as_deref()),
        (Some(50), Some("mira"))
    );
}

#[test]
fn fast_forward_takes_a_whole_number_of_hours_from_one() {
    for hours in ["0", "-3", "1.5", "lots"] {
        let error = parse(&["--fast-forward", hours]).expect_err("refused");
        assert!(error.contains(hours) && error.contains("hours"), "{error}");
    }
    for flag in ["--fast-forward", "--load", "--save"] {
        assert!(parse(&[flag]).expect_err("needs a value").contains(flag));
    }
}

#[test]
fn fast_forward_refuses_more_hours_than_a_run_can_count() {
    // M2 design §16.3: at 4,500 ticks an hour, counted in tenths for the
    // report's rows, a run's ticks must fit in a tick count. The most is
    // u64::MAX / 45,000.
    for hours in [
        "18446744073709551615",
        "4100000000000000",
        "409927646082435",
    ] {
        let error = parse(&["--fast-forward", hours]).expect_err("refused");
        assert!(
            error.contains(hours) && error.contains("409,927,646,082,434"),
            "{error}"
        );
    }
    let most = parse(&["--fast-forward", "409927646082434"]).expect("the most there can be");
    assert_eq!(most.fast_forward, Some(409_927_646_082_434));
}

#[test]
fn a_saved_world_refuses_the_flags_that_make_a_new_one() {
    for flag in [
        &["--seed", "1"][..],
        &["--preset", "p.ron"],
        &["--data", "mods"],
        &["--lab"],
    ] {
        let flags: Vec<&str> = ["--load", "mira"].iter().chain(flag).copied().collect();
        let error = parse(&flags).expect_err("refused");
        assert!(
            error.contains("--load") && error.contains(flag[0]),
            "{error}"
        );
    }
}

#[test]
fn a_replay_refuses_loading_and_fast_forwarding_too() {
    for flags in [
        &["--replay", "a.replay", "--load", "mira"][..],
        &["--replay", "a.replay", "--fast-forward", "5"],
        &["--replay", "a.replay", "--lab"],
    ] {
        let error = parse(flags).expect_err("refused");
        assert!(
            error.contains("--replay") && error.contains(flags[2]),
            "{error}"
        );
    }
}

#[test]
fn save_names_only_what_a_fast_forward_writes() {
    let error = parse(&["--seed", "7", "--save", "evolved"]).expect_err("refused");
    assert!(
        error.contains("--save") && error.contains("--fast-forward"),
        "{error}"
    );
}
