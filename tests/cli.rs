//! Command-line behaviour of the `nullfs` binary that needs no mount.

use std::process::{Command, Output};

fn nullfs(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nullfs"))
        .args(args)
        .output()
        .expect("failed to run nullfs binary")
}

#[test]
fn help_describes_arguments() {
    let out = nullfs(&["--help"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("MOUNT"), "{stdout}");
    assert!(stdout.contains("--option"), "{stdout}");
}

#[test]
fn version_matches_manifest() {
    let out = nullfs(&["--version"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")), "{stdout}");
}

#[test]
fn missing_mount_point_is_a_usage_error() {
    let out = nullfs(&[]);
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("MOUNT"), "{stderr}");
}

#[test]
fn option_flag_requires_a_value() {
    let out = nullfs(&["/mnt", "-o"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn unknown_flag_is_rejected() {
    let out = nullfs(&["/mnt", "--bogus"]);
    assert_eq!(out.status.code(), Some(2));
}
