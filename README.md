# base32

This library lets you encode and decode various Base32 variants. `#[no_std]` compatible and optionally uses the `alloc` crate.

# Usage

```rust
// Crockford's Base32
assert_eq!(base32::CROCKFORD.encode(&[0xF8, 0x3E, 0x0F, 0x83, 0xE0]), "Z0Z0Z0Z0");
assert_eq!(base32::CROCKFORD.decode("Z0Z0Z0Z0").unwrap(), vec![0xF8, 0x3E, 0x0F, 0x83, 0xE0]);

// RFC4648
assert_eq!(base32::RFC4648.encode(&[0xF8, 0x3E, 0x7F, 0x83, 0xE7]), "7A7H7A7H");
assert_eq!(base32::RFC4648.decode("7A7H7A7H").unwrap(), vec![0xF8, 0x3E, 0x7F, 0x83, 0xE7]);

// RFC4648 base32hex
assert_eq!(base32::RFC4648_HEX.encode(&[0xF8, 0x3E, 0x7F, 0x83, 0xE7]), "V0V7V0V7");
assert_eq!(base32::RFC4648_HEX.decode("V0V7V0V7").unwrap(), vec![0xF8, 0x3E, 0x7F, 0x83, 0xE7]);

// z-base-32
assert_eq!(base32::Z.encode(&[0xF8, 0x3E, 0x7F, 0x83, 0xE7]), "9y989y98");
assert_eq!(base32::Z.decode("9y989y98").unwrap(), vec![0xF8, 0x3E, 0x7F, 0x83, 0xE7]);
```

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
