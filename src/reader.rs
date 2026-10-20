//! Dump core: read any [`io::Read`] source in [`BYTES_PER_LINE`]-byte
//! chunks and write formatted lines to any [`io::Write`] sink.
//!
//! The core is generic over `impl `[`io::Read`] sources (no wrapper
//! `Read` impl) and never produces a short line except at true end of
//! input: short reads are accumulated into the chunk until it is full
//! or `read()` returns `Ok(0)`, which is treated as EOF so a healthy
//! partial tail is not an error.

use std::io;

use crate::format::{self, BYTES_PER_LINE};

/// Length of a complete 16-byte line including the newline:
/// 8 offset digits + `": "` + 47 hex-field bytes + 2 separator spaces
/// + 16 gutter bytes + 1 newline.
const FULL_LINE_LEN: usize = 8 + 2 + (BYTES_PER_LINE * 3 - 1) + 2 + BYTES_PER_LINE + 1;

/// Dump all of `src` to `out`, returning the number of bytes consumed.
pub fn dump<R: io::Read, W: io::Write>(src: &mut R, out: &mut W) -> io::Result<u64> {
    let mut chunk = [0u8; BYTES_PER_LINE];
    let mut filled = 0usize;
    let mut offset: u64 = 0;
    let mut line = Vec::with_capacity(FULL_LINE_LEN);
    loop {
        while filled < BYTES_PER_LINE {
            match src.read(&mut chunk[filled..]) {
                // `Ok(0)` is EOF: a partial tail is not an error.
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        if filled == 0 {
            return Ok(offset);
        }
        line.clear();
        format::push_line(&mut line, offset, &chunk[..filled]);
        out.write_all(&line)?;
        offset += filled as u64;
        if filled < BYTES_PER_LINE {
            return Ok(offset);
        }
        filled = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// One full line at offset 0 for the given chunk.
    fn expected(chunk: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        format::push_line(&mut out, 0, chunk);
        out
    }

    fn dump_to_out(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let n = dump(&mut Cursor::new(data), &mut out).expect("dump must succeed");
        assert_eq!(n, data.len() as u64);
        out
    }

    #[test]
    fn empty_input_dumps_nothing() {
        assert_eq!(dump_to_out(b""), b"");
    }

    #[test]
    fn one_byte_input_dumps_one_short_line() {
        assert_eq!(dump_to_out(&[0xff]), b"00000000: ff  .\n");
    }

    #[test]
    fn fifteen_byte_input_dumps_one_short_line() {
        let data = [b'a'; 15];
        assert_eq!(dump_to_out(&data), expected(&data));
    }

    #[test]
    fn sixteen_byte_input_dumps_one_full_line() {
        let data = [b'z'; BYTES_PER_LINE];
        assert_eq!(dump_to_out(&data), expected(&data));
    }

    #[test]
    fn seventeen_byte_input_dumps_full_line_plus_tail() {
        let data = [b'q'; BYTES_PER_LINE + 1];
        let mut want = expected(&data[..BYTES_PER_LINE]);
        let mut tail = Vec::new();
        format::push_line(&mut tail, BYTES_PER_LINE as u64, &data[BYTES_PER_LINE..]);
        want.extend_from_slice(&tail);
        assert_eq!(dump_to_out(&data), want);
    }

    #[test]
    fn thirty_two_byte_input_dumps_two_full_lines() {
        let data = [0xa5u8; BYTES_PER_LINE * 2];
        let mut want = expected(&data[..BYTES_PER_LINE]);
        let mut second = Vec::new();
        format::push_line(&mut second, BYTES_PER_LINE as u64, &data[BYTES_PER_LINE..]);
        want.extend_from_slice(&second);
        assert_eq!(dump_to_out(&data), want);
    }

    /// Source that hands out at most 3 bytes per `read` call.
    struct Trickle<'a> {
        data: &'a [u8],
        pos: usize,
    }

    impl io::Read for Trickle<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let remaining = &self.data[self.pos..];
            if remaining.is_empty() || buf.is_empty() {
                return Ok(0);
            }
            let n = remaining.len().min(buf.len()).min(3);
            buf[..n].copy_from_slice(&remaining[..n]);
            self.pos += n;
            Ok(n)
        }
    }

    #[test]
    fn short_read_source_still_yields_full_lines() {
        let data: Vec<u8> = (0..40u8).collect();
        let mut src = Trickle {
            data: &data,
            pos: 0,
        };
        let mut out = Vec::new();
        let n = dump(&mut src, &mut out).expect("dump must succeed");
        assert_eq!(n, data.len() as u64);
        let mut want = Vec::new();
        format::push_line(&mut want, 0, &data[..16]);
        format::push_line(&mut want, 16, &data[16..32]);
        format::push_line(&mut want, 32, &data[32..]);
        assert_eq!(out, want);
    }

    /// Source that yields `left` `x` bytes and then fails.
    struct FailingAfter {
        left: usize,
    }

    impl io::Read for FailingAfter {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.left == 0 {
                return Err(io::Error::other("boom"));
            }
            let n = self.left.min(buf.len());
            for slot in buf.iter_mut().take(n) {
                *slot = b'x';
            }
            self.left -= n;
            Ok(n)
        }
    }

    #[test]
    fn read_error_propagates_after_complete_lines() {
        let mut out = Vec::new();
        let err = dump(&mut FailingAfter { left: 20 }, &mut out).expect_err("must fail");
        assert_eq!(err.to_string(), "boom");
        // The full first line was already written before the failure.
        assert_eq!(out, expected(&[b'x'; BYTES_PER_LINE]));
    }

    /// Writer that rejects every write like a closed downstream pipe.
    struct BrokenPipeSink;

    impl io::Write for BrokenPipeSink {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed pipe"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn broken_pipe_write_errors_propagate_to_the_caller() {
        let mut src = Cursor::new([0u8; 4]);
        let err = dump(&mut src, &mut BrokenPipeSink).expect_err("must fail");
        assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
    }
}
