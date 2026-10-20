//! Thin `hexy` binary: parse argv, wire the source to a buffered
//! stdout sink through the shared dump core, and map every failure to
//! a single `hexy: `-prefixed stderr line plus exit code 1. Partial
//! stdout is flushed before the error message; EPIPE from a closed
//! downstream pipe is tolerated as normal termination (exit 0).

use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::process::ExitCode;

use hexy::argv::{self, Source};
use hexy::reader;

const USAGE_LINE: &str = "hexy: usage: hexy [<file>]";

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    let source = match argv::parse(args) {
        Ok(source) => source,
        Err(argv::UsageError) => {
            eprintln!("{USAGE_LINE}");
            return ExitCode::from(1);
        }
    };
    let label = source.label();
    match run(&source) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("hexy: {label}: {err}");
            ExitCode::from(1)
        }
    }
}

fn run(source: &Source) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let result = match source {
        Source::Stdin => {
            let mut stdin = io::stdin().lock();
            reader::dump(&mut stdin, &mut out)
        }
        Source::File(path) => match File::open(path) {
            Ok(mut file) => reader::dump(&mut file, &mut out),
            Err(err) => Err(err),
        },
    }
    .map(|_| ());
    match result {
        Ok(()) => out.flush(),
        Err(err) => {
            // Push any partial stdout out before the caller reports the error.
            let _ = out.flush();
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::USAGE_LINE;

    #[test]
    fn usage_line_is_hexy_prefixed_and_self_contained() {
        assert_eq!(USAGE_LINE, "hexy: usage: hexy [<file>]");
    }
}
