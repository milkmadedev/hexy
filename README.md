# hexy

`hexy` is a small command-line tool. It shows the bytes of a file in hex.
It prints one line for every 16 bytes:

- an 8-digit lowercase hex offset, with leading zeros
- the 16 bytes as space-separated lowercase hex pairs
- the same bytes as ASCII text. Printable bytes (0x20 to 0x7E) show as
  they are. All other bytes show as a dot.

The last line can have fewer than 16 bytes.

## Usage

```
hexy [<file>]
```

Give no file, or give `-`, to read standard input.

## Errors

Bad usage, a missing file, or a read error gives one message on
standard error and an exit code of 1. If the next program in a pipe
closes early, `hexy` ends normally with exit code 0.

## Example

```
$ printf 'Hi!\n' | hexy
00000000: 48 69 21 0a  Hi!.
```

## Build

You need Rust 1.70 or later.

```
cargo build --release
cargo test
```

The release binary is in `target/release/hexy`. `hexy` has no
dependencies.

## License

MIT. See the `LICENSE` file.
