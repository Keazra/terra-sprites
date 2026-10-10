//! Records the commit the game is built from, for `--version` (design §6.7).

use std::path::Path;
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn main() {
    let commit = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=TERRA_COMMIT={commit}");

    // Build again when HEAD moves: the file naming the branch, the packed
    // refs and the folder of branch files, watched whole so a branch that was
    // only packed counts when a commit gives it a file of its own. A path that
    // doesn't exist is left out, since cargo would otherwise rerun this on
    // every build.
    for name in ["HEAD", "packed-refs", "refs/heads"] {
        if let Some(path) = git(&["rev-parse", "--git-path", name])
            && Path::new(&path).exists()
        {
            println!("cargo:rerun-if-changed={path}");
        }
    }
}
