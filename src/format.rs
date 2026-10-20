//! Bytes-only line formatting for `hexy`.
//!
//! One line covers [`BYTES_PER_LINE`] bytes and follows the grammar
//!
//! ```text
//! xxxxxxxx: hh hh .. hh  <ASCII gutter>
//! ```
//!
//! where the offset is printed as at least eight lowercase hexadecimal
//! digits (widening past 32-bit offsets instead of wrapping), the hex
//! field is space-separated lowercase pairs, and the gutter shows each
//! byte as-is when it is printable ASCII (`0x20..=0x7e`) and as `.`
//! otherwise. A short final line keeps the standard two-space separator
//! but is not padded: the gutter ends after as many characters as there
//! are bytes in the chunk.

/// Number of payload bytes covered by one dump line.
pub const BYTES_PER_LINE: usize = 16;

/// Lowercase hexadecimal digit lookup table.
const HEX_DIGITS: [u8; 16] = *b"0123456789abcdef";

/// Map one byte to its gutter representation: printable ASCII as-is,
/// everything else (including `0x7f` and all high bytes) as `.`.
pub fn gutter_byte(byte: u8) -> u8 {
    if (0x20..=0x7e).contains(&byte) {
        byte
    } else {
        b'.'
    }
}

/// Append `value` to `out` as lowercase hexadecimal digits,
/// zero-padded to at least `min_width` digits.
fn push_hex(out: &mut Vec<u8>, value: u64, min_width: usize) {
    // `min_width` is a floor, not a fixed column count: offsets beyond
    // 4 GiB widen past eight digits instead of wrapping.
    let mut digits = [0u8; 16];
    let mut len = 0;
    let mut rest = value;
    loop {
        digits[len] = HEX_DIGITS[(rest & 0xf) as usize];
        len += 1;
        rest >>= 4;
        if rest == 0 {
            break;
        }
    }
    for _ in len..min_width {
        out.push(b'0');
    }
    for index in (0..len).rev() {
        out.push(digits[index]);
    }
}

/// Format one dump line covering `chunk` (at most [`BYTES_PER_LINE`]
/// bytes) starting at `offset`, appending it with its trailing newline
/// to `out`.
pub fn push_line(out: &mut Vec<u8>, offset: u64, chunk: &[u8]) {
    debug_assert!(chunk.len() <= BYTES_PER_LINE);

    push_hex(out, offset, 8);
    out.push(b':');
    out.push(b' ');
    for (index, &byte) in chunk.iter().enumerate() {
        if index > 0 {
            out.push(b' ');
        }
        out.push(HEX_DIGITS[(byte >> 4) as usize]);
        out.push(HEX_DIGITS[(byte & 0x0f) as usize]);
    }
    out.push(b' ');
    out.push(b' ');
    for &byte in chunk {
        out.push(gutter_byte(byte));
    }
    out.push(b'\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(offset: u64, chunk: &[u8]) -> String {
        let mut out = Vec::new();
        push_line(&mut out, offset, chunk);
        String::from_utf8(out).expect("line is ASCII")
    }

    #[test]
    fn golden_full_line_at_offset_zero() {
        let chunk = b"0123456789abcdef";
        assert_eq!(
            line(0, chunk),
            "00000000: 30 31 32 33 34 35 36 37 38 39 61 62 63 64 65 66  0123456789abcdef\n"
        );
    }

    #[test]
    fn golden_full_line_at_offset_0x10() {
        assert_eq!(
            line(0x10, &[0u8; BYTES_PER_LINE]),
            "00000010: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  ................\n"
        );
    }

    #[test]
    fn golden_offset_0xffff_fff0_short_tail() {
        assert_eq!(line(0xffff_fff0, b"ab"), "fffffff0: 61 62  ab\n");
    }

    #[test]
    fn golden_offset_widens_past_32_bits_instead_of_wrapping() {
        assert_eq!(line(0x1_0000_0000, b"z"), "100000000: 7a  z\n");
    }

    #[test]
    fn golden_one_byte_and_fifteen_byte_tails_keep_two_space_separator() {
        assert_eq!(line(0, &[0xff]), "00000000: ff  .\n");
        assert_eq!(
            line(0, b"abcdefghijklmno"),
            "00000000: 61 62 63 64 65 66 67 68 69 6a 6b 6c 6d 6e 6f  abcdefghijklmno\n"
        );
    }

    #[test]
    fn gutter_table_covers_all_256_byte_values() {
        for byte in 0..=255u8 {
            let expected = if (0x20..=0x7e).contains(&byte) {
                byte
            } else {
                b'.'
            };
            assert_eq!(gutter_byte(byte), expected, "mapping of 0x{byte:02x}");
        }
    }

    #[test]
    fn push_hex_zero_pads_to_the_minimum_width() {
        let mut out = Vec::new();
        push_hex(&mut out, 0, 8);
        assert_eq!(out, b"00000000");

        let mut out = Vec::new();
        push_hex(&mut out, 0xcafe, 8);
        assert_eq!(out, b"0000cafe");
    }
}
