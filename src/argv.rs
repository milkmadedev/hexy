//! Command-line argument parsing: zero arguments or a single `-`
//! selects stdin, exactly one other argument selects a file path, and
//! any other argument count is a usage error.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

/// Where the dump input comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Read standard input (no argument, or `-`).
    Stdin,
    /// Read this file.
    File(PathBuf),
}

/// The argument vector did not match the accepted usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsageError;

impl Source {
    /// Label used in error messages for this source.
    pub fn label(&self) -> String {
        match self {
            Source::Stdin => "stdin".to_string(),
            Source::File(path) => path.display().to_string(),
        }
    }
}

/// Parse the arguments following the program name.
pub fn parse<I>(args: I) -> Result<Source, UsageError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let only = match args.next() {
        Some(arg) => arg,
        None => return Ok(Source::Stdin),
    };
    if args.next().is_some() {
        return Err(UsageError);
    }
    if only.as_os_str() == OsStr::new("-") {
        return Ok(Source::Stdin);
    }
    Ok(Source::File(PathBuf::from(only)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<Source, UsageError> {
        parse(args.iter().map(OsString::from))
    }

    #[test]
    fn no_args_reads_stdin() {
        assert_eq!(parse_args(&[]), Ok(Source::Stdin));
    }

    #[test]
    fn lone_dash_unconditionally_reads_stdin() {
        assert_eq!(parse_args(&["-"]), Ok(Source::Stdin));
    }

    #[test]
    fn single_argument_is_a_file_path() {
        assert_eq!(
            parse_args(&["file.bin"]),
            Ok(Source::File(PathBuf::from("file.bin")))
        );
    }

    #[test]
    fn two_arguments_is_a_usage_error() {
        assert_eq!(parse_args(&["a.bin", "b.bin"]), Err(UsageError));
    }

    #[test]
    fn dash_plus_argument_is_a_usage_error() {
        assert_eq!(parse_args(&["-", "x"]), Err(UsageError));
    }

    #[test]
    fn labels_are_stdin_and_the_verbatim_path() {
        assert_eq!(Source::Stdin.label(), "stdin");
        assert_eq!(Source::File(PathBuf::from("data.bin")).label(), "data.bin");
    }
}
