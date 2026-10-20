//! Integration tests for the `hexy` CLI: stdin handling and the error
//! paths (bad usage, missing file, mid-stream read error), a streaming
//! line-by-line read, and EPIPE tolerance.

use std::io::{BufRead, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn hexy(args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_hexy"));
    cmd.args(args);
    cmd
}

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("hexy-it-{}-{name}", std::process::id()))
}

fn run_with_stdin(args: &[&str], input: &[u8]) -> std::process::Output {
    let mut child = hexy(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn hexy");
    {
        let mut stdin = child.stdin.take().expect("stdin pipe");
        stdin.write_all(input).expect("feed stdin");
    } // dropping the handle closes the pipe
    child.wait_with_output().expect("wait for hexy")
}

#[test]
fn no_args_dumps_piped_stdin() {
    let out = run_with_stdin(&[], b"hello");
    assert!(out.status.success());
    assert_eq!(out.stdout, b"00000000: 68 65 6c 6c 6f  hello\n");
}

#[test]
fn lone_dash_reads_stdin() {
    let out = run_with_stdin(&["-"], b"AB");
    assert!(out.status.success());
    assert_eq!(out.stdout, b"00000000: 41 42  AB\n");
}

#[test]
fn missing_file_exits_one_with_stderr_message() {
    let out = hexy(&["/definitely/not/a/hexy/fixture"])
        .output()
        .expect("run hexy");
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8(out.stderr).expect("stderr is utf-8");
    assert!(
        stderr.starts_with("hexy: /definitely/not/a/hexy/fixture: "),
        "stderr was: {stderr}"
    );
    assert!(!stderr.contains("usage"), "unexpected usage text: {stderr}");
}

#[test]
fn two_args_exit_one_with_usage_message() {
    let out = hexy(&["a.bin", "b.bin"]).output().expect("run hexy");
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8(out.stderr).expect("stderr is utf-8");
    assert_eq!(stderr, "hexy: usage: hexy [<file>]\n");
}

#[test]
fn unreadable_source_exits_one_with_labelled_error() {
    // A directory opens successfully but fails on read on Unix, exercising
    // the mid-stream read-error path.
    let out = hexy(&["/"]).output().expect("run hexy");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8(out.stderr).expect("stderr is utf-8");
    assert!(stderr.starts_with("hexy: /: "), "stderr was: {stderr}");
}

#[test]
fn broken_downstream_pipe_is_tolerated_as_success() {
    let path = temp_path("epipe.bin");
    std::fs::write(&path, vec![b'e'; 1 << 20]).expect("write fixture");
    let mut child = hexy(&[path.to_str().expect("utf-8 path")])
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn hexy");
    drop(child.stdout.take()); // close the read end so writes hit EPIPE
    let status = child.wait().expect("wait hexy");
    let _ = std::fs::remove_file(&path);
    assert_eq!(status.code(), Some(0));
}

#[test]
fn file_arg_output_is_readable_line_by_line() {
    let path = temp_path("first-line.bin");
    std::fs::write(&path, [0u8; 32]).expect("write fixture");
    let mut child = hexy(&[path.to_str().expect("utf-8 path")])
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn hexy");
    let stdout = child.stdout.take().expect("stdout pipe");
    let mut reader = std::io::BufReader::new(stdout);
    let mut first = Vec::new();
    reader.read_until(b'\n', &mut first).expect("read first line");
    let mut rest = String::new();
    reader.read_to_string(&mut rest).expect("read rest");
    let status = child.wait().expect("wait hexy");
    let _ = std::fs::remove_file(&path);
    assert!(status.success());
    assert_eq!(
        first,
        b"00000000: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  ................\n"
    );
    assert_eq!(
        rest,
        "00000010: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  ................\n"
    );
}
