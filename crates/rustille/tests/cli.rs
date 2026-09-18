//! Black-box tests for the `rustille` binary.
//!
//! These only build when the `cli` feature is on, because that is the only
//! configuration in which the binary target exists.

#![cfg(feature = "cli")]

mod common;

use std::io::Write;
use std::process::{Command, Stdio};

use image::ImageFormat;

const BIN: &str = env!("CARGO_BIN_EXE_rustille");

struct Output {
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run(args: &[&str], stdin: Option<&[u8]>) -> Output {
    let mut command = Command::new(BIN);
    command
        .args(args)
        .env_remove("COLORTERM")
        .env_remove("NO_COLOR")
        .env("TERM", "xterm-256color")
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().expect("the binary is executable");
    if let Some(bytes) = stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin was piped")
            .write_all(bytes)
            .expect("the child accepts input");
    }
    let output = child.wait_with_output().expect("the child terminates");
    Output {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn sample_png() -> Vec<u8> {
    common::encode(&common::checkerboard(64, 64, 8), ImageFormat::Png)
}

fn is_braille(text: &str) -> bool {
    !text.trim().is_empty()
        && text
            .trim_end()
            .chars()
            .all(|c| c == '\n' || rustille::braille::mask_for_char(c).is_some())
}

#[test]
fn version_and_help_succeed() {
    let version = run(&["--version"], None);
    assert_eq!(version.status, Some(0));
    assert!(version.stdout.contains(env!("CARGO_PKG_VERSION")));

    let help = run(&["--help"], None);
    assert_eq!(help.status, Some(0));
    assert!(help.stdout.contains("--width"));
    assert!(help.stdout.contains("--dither"));
    assert!(help.stdout.contains("--color"));
    assert!(help.stdout.contains("Braille"));
}

#[test]
fn renders_a_file_to_stdout() {
    let path = common::write_fixture("cli-checkerboard.png", &sample_png());
    let output = run(&[path.to_str().unwrap(), "--width", "20"], None);

    assert_eq!(output.status, Some(0), "stderr: {}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {}", output.stderr);
    assert!(is_braille(&output.stdout), "{:?}", output.stdout);
    assert!(output.stdout.ends_with('\n'));
    for line in output.stdout.lines() {
        assert_eq!(line.chars().count(), 20);
    }
}

#[test]
fn honours_the_documented_flags() {
    let path = common::write_fixture("cli-flags.png", &sample_png());
    let file = path.to_str().unwrap();

    let tall = run(&[file, "--height", "10"], None);
    assert_eq!(tall.status, Some(0));
    assert_eq!(tall.stdout.lines().count(), 10);

    let threshold = run(&[file, "--width", "10", "--threshold", "250"], None);
    assert_eq!(threshold.status, Some(0));

    let inverted = run(&[file, "--width", "10", "--invert"], None);
    let plain = run(&[file, "--width", "10"], None);
    assert_ne!(inverted.stdout, plain.stdout);

    let dithered = run(
        &[file, "--width", "10", "--dither", "floyd-steinberg"],
        None,
    );
    assert_eq!(dithered.status, Some(0));
    assert!(is_braille(&dithered.stdout));

    let stretched = run(
        &[file, "--width", "10", "--height", "10", "--fit", "stretch"],
        None,
    );
    assert_eq!(stretched.stdout.lines().count(), 10);

    let contained = run(
        &[file, "--width", "10", "--height", "10", "--fit", "contain"],
        None,
    );
    assert_eq!(contained.stdout.lines().count(), 5);
}

#[test]
fn reads_an_image_from_stdin() {
    let bytes = sample_png();
    let piped = run(&["-", "--width", "20"], Some(&bytes));
    assert_eq!(piped.status, Some(0), "stderr: {}", piped.stderr);

    let path = common::write_fixture("cli-stdin.png", &bytes);
    let from_file = run(&[path.to_str().unwrap(), "--width", "20"], None);
    assert_eq!(piped.stdout, from_file.stdout);
}

#[test]
fn colour_is_off_when_piped_and_on_when_forced() {
    let path = common::write_fixture("cli-colour.png", &sample_png());
    let file = path.to_str().unwrap();

    let auto = run(&[file, "--width", "10"], None);
    assert!(!auto.stdout.contains('\u{1b}'));

    let always = run(&[file, "--width", "10", "--color", "always"], None);
    assert!(always.stdout.contains("\x1b[38;5;"), "{:?}", always.stdout);

    let truecolor = run(&[file, "--width", "10", "--color", "truecolor"], None);
    assert!(truecolor.stdout.contains("\x1b[38;2;"));

    let never = run(&[file, "--width", "10", "--color", "never"], None);
    assert!(!never.stdout.contains('\u{1b}'));
}

#[test]
fn writes_to_a_file_with_output() {
    let path = common::write_fixture("cli-output-source.png", &sample_png());
    let destination = common::scratch_dir().join("cli-output.txt");
    let _ = std::fs::remove_file(&destination);

    let output = run(
        &[
            path.to_str().unwrap(),
            "--width",
            "12",
            "--output",
            destination.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(output.status, Some(0), "stderr: {}", output.stderr);
    assert!(output.stdout.is_empty());

    let written = std::fs::read_to_string(&destination).expect("the output file exists");
    assert!(is_braille(&written));
}

#[test]
fn a_missing_file_exits_with_one_and_writes_to_stderr() {
    let missing = common::scratch_dir().join("not-here.png");
    let output = run(&[missing.to_str().unwrap()], None);

    assert_eq!(output.status, Some(1));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.starts_with("rustille: "), "{}", output.stderr);
    assert!(output.stderr.contains("failed to read"));
}

#[test]
fn undecodable_input_exits_with_one() {
    let path = common::write_fixture("cli-garbage.png", b"this is not a png");
    let output = run(&[path.to_str().unwrap()], None);
    assert_eq!(output.status, Some(1));
    assert!(output.stderr.contains("failed to render"));
}

#[test]
fn empty_stdin_exits_with_one() {
    let output = run(&["-"], Some(b""));
    assert_eq!(output.status, Some(1));
    assert!(output.stderr.contains("no image data"));
}

#[test]
fn bad_usage_exits_with_two() {
    for args in [
        vec!["--definitely-not-a-flag"],
        vec!["image.png", "--threshold", "999"],
        vec!["image.png", "--dither", "atkinson"],
        vec![],
    ] {
        let output = run(&args, None);
        assert_eq!(output.status, Some(2), "args: {args:?}");
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn an_invalid_background_exits_with_one() {
    let path = common::write_fixture("cli-background.png", &sample_png());
    let output = run(
        &[path.to_str().unwrap(), "--background", "chartreuse"],
        None,
    );
    assert_eq!(output.status, Some(1));
    assert!(output.stderr.contains("background"));
}
