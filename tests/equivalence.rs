//! Byte-identical output across file, piped-stdin (no arg) and `-`
//! sources, plus corpus line counts. Corpora are generated inline at
//! test time (including the 10,000,000-byte one); nothing is committed.

use hexy::reader;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("hexy-it-{}-{name}", std::process::id()))
}

fn run_file(path: &Path) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_hexy"))
        .arg(path)
        .output()
        .expect("run hexy on file");
    assert!(output.status.success(), "hexy {path:?} failed");
    output.stdout
}

fn run_stdin(args: &[&str], data: &[u8]) -> Vec<u8> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_hexy"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn hexy");
    {
        let mut stdin = child.stdin.take().expect("stdin pipe");
        stdin.write_all(data).expect("feed stdin");
    }
    let output = child.wait_with_output().expect("wait for hexy");
    assert!(
        output.status.success(),
        "hexy {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn pseudo_random(len: usize) -> Vec<u8> {
    let mut state = 0x0123_4567_89ab_cdefu64;
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        out.push((state >> 33) as u8);
    }
    out
}

/// Sink that counts complete newline-terminated lines.
struct LineCounter(usize);

impl io::Write for LineCounter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 += buf.iter().filter(|&&b| b == b'\n').count();
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn file_piped_stdin_and_dash_outputs_are_byte_identical() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", Vec::new()),
        ("seventeen", b"abcdefghijklmnopq".to_vec()),
        ("all256", (0..=255u8).collect()),
        ("prnd1000", pseudo_random(1000)),
    ];
    for (name, data) in &cases {
        let path = temp_path(&format!("equiv-{name}.bin"));
        std::fs::write(&path, data).expect("write fixture");
        let mut src = Cursor::new(data.as_slice());
        let mut expected = Vec::new();
        reader::dump(&mut src, &mut expected).expect("lib dump");
        assert_eq!(run_file(&path), expected, "file source: {name}");
        assert_eq!(run_stdin(&[], data), expected, "no-arg stdin: {name}");
        assert_eq!(run_stdin(&["-"], data), expected, "'-' stdin: {name}");
        let _ = std::fs::remove_file(&path);
    }
}

#[test]
fn corpus_line_counts() {
    let cases = [
        (0usize, 0usize),
        (16, 1),
        (32, 2),
        (17, 2),
        (10_000_000, 625_000),
    ];
    for (size, want) in cases {
        let data = pseudo_random(size);
        let mut counter = LineCounter(0);
        reader::dump(&mut Cursor::new(data.as_slice()), &mut counter).expect("dump corpus");
        assert_eq!(counter.0, want, "lines for {size} bytes");
    }
}
