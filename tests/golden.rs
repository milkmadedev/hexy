//! Golden comparisons pinning hexy's byte-exact line grammar. All
//! fixtures are built inline; nothing is committed as a fixture file.

use hexy::reader;
use std::io::Cursor;

fn dump(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    reader::dump(&mut Cursor::new(data), &mut out).expect("lib dump");
    out
}

/// Independent reference implementation of the documented grammar,
/// used to cross-check the library on larger corpora.
fn reference(data: &[u8]) -> String {
    let mut text = String::new();
    for (index, chunk) in data.chunks(16).enumerate() {
        text.push_str(&format!("{:08x}: ", index * 16));
        let pairs: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        text.push_str(&pairs.join(" "));
        text.push_str("  ");
        for &b in chunk {
            text.push(if (0x20..=0x7e).contains(&b) {
                b as char
            } else {
                '.'
            });
        }
        text.push('\n');
    }
    text
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

#[test]
fn empty_input_dumps_nothing() {
    assert_eq!(dump(b""), b"");
}

#[test]
fn printable_boundary_bytes_map_exactly() {
    // 0x1f -> '.', 0x20 -> ' ', 0x7e -> '~', 0x7f -> '.'.
    let data = [0x1f, 0x20, 0x7e, 0x7f];
    assert_eq!(
        String::from_utf8(dump(&data)).expect("dump output is ASCII"),
        "00000000: 1f 20 7e 7f  . ~.\n"
    );
    assert_eq!(dump(&data), reference(&data).as_bytes());
}

#[test]
fn five_byte_tail_is_unpadded() {
    let data = b"Hello";
    assert_eq!(
        String::from_utf8(dump(data)).expect("dump output is ASCII"),
        "00000000: 48 65 6c 6c 6f  Hello\n"
    );
}

#[test]
fn full_line_of_bytes_zero_to_fifteen_matches_reference() {
    let data: Vec<u8> = (0..16u8).collect();
    let text = String::from_utf8(dump(&data)).expect("dump output is ASCII");
    assert_eq!(
        text,
        "00000000: 00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f  ................\n"
    );
    assert_eq!(text, reference(&data));
}

#[test]
fn seventeen_bytes_match_reference_with_short_second_line() {
    let data = b"abcdefghijklmnopq";
    let text = String::from_utf8(dump(data)).expect("dump output is ASCII");
    assert_eq!(
        text,
        "00000000: 61 62 63 64 65 66 67 68 69 6a 6b 6c 6d 6e 6f 70  abcdefghijklmnop\n\
         00000010: 71  q\n"
    );
    assert_eq!(dump(data), reference(data).as_bytes());
}

#[test]
fn all_256_byte_values_match_reference_and_full_gutter_mapping() {
    let data: Vec<u8> = (0..=255u8).collect();
    let text = String::from_utf8(dump(&data)).expect("dump output is ASCII");
    assert_eq!(text, reference(&data));
    for (index, line) in text.lines().enumerate() {
        let gutter = &line[line.len() - 16..];
        let expected: String = (0..16)
            .map(|k| {
                let byte = (index * 16 + k) as u8;
                if (0x20..=0x7e).contains(&byte) {
                    byte as char
                } else {
                    '.'
                }
            })
            .collect();
        assert_eq!(gutter, expected, "gutter of line {index}");
    }
}

#[test]
fn deterministic_pseudo_random_corpus_matches_reference() {
    let data = pseudo_random(5000);
    assert_eq!(dump(&data), reference(&data).as_bytes());
}

#[test]
fn offsets_progress_by_sixteen() {
    let data = pseudo_random(33);
    let text = String::from_utf8(dump(&data)).expect("dump output is ASCII");
    let offsets: Vec<&str> = text.lines().map(|line| &line[..8]).collect();
    assert_eq!(offsets, ["00000000", "00000010", "00000020"]);
}
