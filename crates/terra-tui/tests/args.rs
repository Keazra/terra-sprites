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
