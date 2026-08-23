#![no_std]
#![deny(missing_docs)]

//! Encoders and decoders for various Base32 variants.
//!
//! # Examples
//! Predefined encodings are provided as constants of type `Encoding` which provides encoding and decoding functions.
//! Both owned and in-place functions are available.
//!
//! ```rust
//! let output = base32::RFC4648.encode(b"Hello, world!");
//! assert_eq!(output, "JBSWY3DPFQQHO33SNRSCC===");
//! ```
//!
//! ```rust
//! let mut output = vec![0; 13];
//! base32::CROCKFORD.decode_buf("91JPRV3F5GG7EVVJDHJ22", &mut output).unwrap();
//! assert_eq!(output, b"Hello, world!");
//! ```
//!
//! You can also define your own Base32 encoding.
//! ```rust
//! let encoding = base32::Encoding::new(*b"0123456789aBcDeFgHiJkLmNoPqRsTuV")
//!     .with_symbol(b'$', 28)
//!     .with_symbol(b'w', 31)
//!     .with_padding(b'+')
//!     .with_ignore_case()
//!     .with_reversed();
//! let output = encoding.encode(b"Hello, world!");
//! assert_eq!(output, "45i6osJFesg2oRRcDHikg+++");
//! let output = encoding.decode("45i6O$JFESG2ORRCDHikg+++").unwrap();
//! assert_eq!(output, b"Hello, world!");
//! ```

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::fmt::Display;

/// A Base32 encoding.
///
/// There are predefined encodings available at the crate root like [`RFC4846`] and you can also define your own custom encodings.
pub struct Encoding {
    alphabet: [u8; 32],
    inv_alphabet: [u8; 128],
    padding: Option<u8>,
    reversed: bool,
}

impl Encoding {
    /// Create a new Base32 encoding.
    ///
    /// The passed alphabet is always used for encoding.
    ///
    /// By default padding is disabled and the bytes are encoded left-to-right.
    pub const fn new(alphabet: [u8; 32]) -> Self {
        let mut encoder = Self {
            alphabet,
            inv_alphabet: [255; _],
            padding: None,
            reversed: false,
        };

        let mut value = 0;
        while value < alphabet.len() {
            encoder = encoder.with_symbol(alphabet[value], value as u8);
            value += 1;
        }

        encoder
    }

    /// Add an extra symbol that the decoder will recognise.
    ///
    /// This function panics if `symbol` is greater than 127 or `value` is greater than 31;
    pub const fn with_symbol(mut self, symbol: u8, value: u8) -> Self {
        assert!(symbol < 128);
        assert!(value < 32);

        self.inv_alphabet[symbol as usize] = value;

        self
    }

    /// Make the encoding case-insensitive.
    ///
    /// Note: symbols added after this function is called will still be case-sensitive.
    pub const fn with_ignore_case(mut self) -> Self {
        let mut c = 0u8;

        while c < 128 {
            if self.inv_alphabet[c as usize] != 255 {
                if c.is_ascii_lowercase() {
                    self.inv_alphabet[c.to_ascii_uppercase() as usize] =
                        self.inv_alphabet[c as usize];
                } else if c.is_ascii_uppercase() {
                    self.inv_alphabet[c.to_ascii_lowercase() as usize] =
                        self.inv_alphabet[c as usize];
                }
            }

            c += 1;
        }

        self
    }

    /// Specify the padding symbol used by this encoding.
    ///
    /// By default no padding is used.
    pub const fn with_padding(mut self, padding: u8) -> Self {
        self.padding = Some(padding);

        self
    }

    /// Process the input from right-to-left.
    pub const fn with_reversed(mut self) -> Self {
        self.reversed = true;

        self
    }

    /// Get the alphabet used by this encoding.
    ///
    /// Does not return extra symbols added with [`with_symbol()`].
    pub const fn alphabet(&self) -> &[u8] {
        &self.alphabet
    }

    /// Get the length of the buffer needed to encode an input of `length` bytes.
    ///
    /// Useful for determining the size of the buffer needed for [`encode_buf()`].
    pub fn encoded_length(&self, length: usize) -> usize {
        let num_chunks = length.div_ceil(5);
        let padding_len = if length % 5 > 0 {
            8 - (length % 5 * 8).div_ceil(5)
        } else {
            0
        };

        if self.padding.is_some() {
            num_chunks * 8
        } else {
            num_chunks * 8 - padding_len
        }
    }

    /// Encode `data` with this encoding.
    #[cfg(feature = "alloc")]
    pub fn encode(&self, data: &[u8]) -> String {
        let mut ret = alloc::vec![0; self.encoded_length(data.len())];

        self.encode_buf(data, &mut ret).unwrap();

        String::from_utf8(ret).unwrap()
    }

    fn encode_chunk(&self, chunk: [u8; 5]) -> [u8; 8] {
        [
            self.alphabet[((chunk[0] & 0xF8) >> 3) as usize],
            self.alphabet[(((chunk[0] & 0x07) << 2) | ((chunk[1] & 0xC0) >> 6)) as usize],
            self.alphabet[((chunk[1] & 0x3E) >> 1) as usize],
            self.alphabet[(((chunk[1] & 0x01) << 4) | ((chunk[2] & 0xF0) >> 4)) as usize],
            self.alphabet[(((chunk[2] & 0x0F) << 1) | (chunk[3] >> 7)) as usize],
            self.alphabet[((chunk[3] & 0x7C) >> 2) as usize],
            self.alphabet[(((chunk[3] & 0x03) << 3) | ((chunk[4] & 0xE0) >> 5)) as usize],
            self.alphabet[(chunk[4] & 0x1F) as usize],
        ]
    }

    /// Encode `data` with this encoding into `buf`.
    pub fn encode_buf<'a>(&self, data: &[u8], buf: &'a mut [u8]) -> Result<&'a mut [u8], Error> {
        let padding_len = if data.len() % 5 != 0 {
            8 - (data.len() % 5 * 8).div_ceil(5)
        } else {
            0
        };
        let encoded_len = self.encoded_length(data.len());

        let Some(buf) = buf.get_mut(..encoded_len) else {
            return Err(Error::BufferTooSmall {
                expected: encoded_len,
            });
        };

        for (dst, chunk) in buf.chunks_mut(8).zip(Chunker::new(data, self.reversed, 0)) {
            dst.copy_from_slice(&self.encode_chunk(chunk)[..dst.len()]);
        }

        if let Some(padding) = self.padding {
            let buf_len = buf.len();
            buf[buf_len - padding_len..].fill(padding);
        }

        Ok(buf)
    }

    /// Get the length of the buffer needed to decode `data`.
    ///
    /// Useful for determining the size of the buffer needed for [`decode_buf()`].
    ///
    /// Returns an error if the length or padding are invalid.
    pub fn decoded_length(&self, data: &str) -> Result<usize, Error> {
        let (full_chunks, padding_len) = if let Some(padding) = self.padding {
            if data.len() % 8 != 0 {
                return Err(Error::InvalidLength);
            }

            let padding_len = data.bytes().rev().take_while(|b| *b == padding).count();

            if padding_len > 0 {
                (data.len() / 8 - 1, padding_len)
            } else {
                (data.len() / 8, 0)
            }
        } else {
            if data.len() % 8 == 0 {
                (data.len() / 8, 0)
            } else {
                (data.len() / 8, 8 - data.len() % 8)
            }
        };

        if ![0, 6, 4, 3, 1].contains(&padding_len) {
            if self.padding.is_some() {
                return Err(Error::InvalidPadding);
            } else {
                return Err(Error::InvalidLength);
            }
        }

        Ok(full_chunks * 5
            + if padding_len > 0 {
                (8 - padding_len) * 5 / 8
            } else {
                0
            })
    }

    #[cfg(feature = "alloc")]
    /// Decode `data` with this encoding.
    pub fn decode(&self, data: &str) -> Result<Vec<u8>, Error> {
        let mut ret = alloc::vec![0; self.decoded_length(data)?];
        self.decode_buf(data, &mut ret)?;
        Ok(ret)
    }

    fn decode_chunk(&self, chunk_index: usize, chunk: [u8; 8]) -> Result<[u8; 5], Error> {
        let decoded = {
            let mut buf = [0u8; 8];
            for (i, c) in chunk.into_iter().enumerate() {
                match self.inv_alphabet.get(c as usize) {
                    Some(&255) | None => {
                        return Err(Error::InvalidSymbol {
                            offset: chunk_index * 8 + i,
                            symbol: c,
                        });
                    }
                    Some(&value) => buf[i] = value,
                };
            }
            buf
        };
        Ok([
            (decoded[0] << 3) | (decoded[1] >> 2),
            (decoded[1] << 6) | (decoded[2] << 1) | (decoded[3] >> 4),
            (decoded[3] << 4) | (decoded[4] >> 1),
            (decoded[4] << 7) | (decoded[5] << 2) | (decoded[6] >> 3),
            (decoded[6] << 5) | decoded[7],
        ])
    }

    /// Decode `data` with this encoding into `buf`.
    pub fn decode_buf<'a>(&self, data: &str, buf: &'a mut [u8]) -> Result<&'a mut [u8], Error> {
        let output_length = self.decoded_length(data)?;
        let Some(ret) = buf.get_mut(..output_length) else {
            return Err(Error::BufferTooSmall {
                expected: output_length,
            });
        };

        let last_chunk_index = data.len().div_ceil(8).saturating_sub(1);

        for (chunk_index, (dst, mut chunk)) in ret
            .chunks_mut(5)
            .zip(Chunker::new(data.as_bytes(), false, self.alphabet()[0]))
            .enumerate()
        {
            if chunk_index == last_chunk_index {
                if let Some(padding) = self.padding {
                    for b in chunk.iter_mut().rev() {
                        if *b == padding {
                            *b = self.alphabet[0];
                        } else {
                            break;
                        }
                    }
                }
            }

            dst.copy_from_slice(&self.decode_chunk(chunk_index, chunk)?[..dst.len()]);
        }

        if self.reversed {
            ret.reverse();
        }

        Ok(ret)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Decoding error
pub enum Error {
    /// Invalid symbol found in input
    InvalidSymbol {
        /// Index of the invalid symbol
        offset: usize,
        /// The invalid symbol
        symbol: u8,
    },
    /// Length of the input is invalid
    InvalidLength,
    /// Padding is invalid
    InvalidPadding,
    /// Passed buffer is too small
    BufferTooSmall {
        /// Minimum buffer size required
        expected: usize,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSymbol { offset, symbol } => {
                core::write!(
                    f,
                    "invalid symbol {} (0x{symbol:02X}) at offset {offset}",
                    char::from(*symbol)
                )
            }
            Self::InvalidLength => f.write_str("invalid length"),
            Self::InvalidPadding => f.write_str("invalid padding"),
            Self::BufferTooSmall { expected } => core::write!(
                f,
                "passed buffer is too small, expected at least {expected} bytes"
            ),
        }
    }
}

impl core::error::Error for Error {}

struct Chunker<'a, const N: usize> {
    slice: &'a [u8],
    reverse: bool,
    pad: u8,
}

impl<'a, const N: usize> Chunker<'a, N> {
    fn new(slice: &'a [u8], reverse: bool, pad: u8) -> Self {
        Self {
            slice,
            reverse,
            pad,
        }
    }
}

impl<const N: usize> Iterator for Chunker<'_, N> {
    type Item = [u8; N];

    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.is_empty() {
            return None;
        }

        let mut ret = [self.pad; N];
        let len = N.min(self.slice.len());

        if !self.reverse {
            let (chunk, rest) = self.slice.split_at(len);
            ret[..len].copy_from_slice(&chunk);
            self.slice = rest;
        } else {
            let (rest, chunk) = self.slice.split_at(self.slice.len() - len);
            ret[..len].copy_from_slice(&chunk);
            ret[..len].reverse();
            self.slice = rest;
        }

        Some(ret)
    }
}

/// [Crockford's Base32 encoding](https://www.crockford.com/base32.html)
pub const CROCKFORD: Encoding = Encoding::new(*b"0123456789ABCDEFGHJKMNPQRSTVWXYZ")
    .with_symbol(b'I', 1)
    .with_symbol(b'L', 1)
    .with_symbol(b'O', 0)
    .with_ignore_case();

/// "base32" encoding according to [RFC 4648 section 6](https://www.rfc-editor.org/rfc/rfc4648#section-6) but without padding.
pub const RFC4648_NOPAD: Encoding = Encoding::new(*b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567");
/// "base32" encoding according to [RFC 4648 section 6](https://www.rfc-editor.org/rfc/rfc4648#section-6).
pub const RFC4648: Encoding = RFC4648_NOPAD.with_padding(b'=');

/// Lowercased version of "base32" according to [RFC 4648 section 6](https://www.rfc-editor.org/rfc/rfc4648#section-6) encoding without padding.
pub const RFC4648_LOWER_NOPAD: Encoding = Encoding::new(*b"abcdefghijklmnopqrstuvwxyz234567");
/// Lowercased version of "base32" according to [RFC 4648 section 6](https://www.rfc-editor.org/rfc/rfc4648#section-6) encoding.
pub const RFC4648_LOWER: Encoding = RFC4648_LOWER_NOPAD.with_padding(b'=');

/// "base32hex" encoding according to [RFC 4648 section 7](https://www.rfc-editor.org/rfc/rfc4648#section-7) but without padding.
pub const RFC4648_HEX_NOPAD: Encoding = Encoding::new(*b"0123456789ABCDEFGHIJKLMNOPQRSTUV");
/// "base32hex" encoding according to [RFC 4648 section 7](https://www.rfc-editor.org/rfc/rfc4648#section-7).
pub const RFC4648_HEX: Encoding = RFC4648_HEX_NOPAD.with_padding(b'=');

/// Lowercased "base32hex" according to [RFC 4648 section 7](https://www.rfc-editor.org/rfc/rfc4648#section-7) but without padding.
pub const RFC4648_HEX_LOWER_NOPAD: Encoding = Encoding::new(*b"0123456789abcdefghijklmnopqrstuv");
/// Lowercased "base32hex" according to [RFC 4648 section 7](https://www.rfc-editor.org/rfc/rfc4648#section-7).
pub const RFC4648_HEX_LOWER: Encoding = RFC4648_HEX_LOWER_NOPAD.with_padding(b'=');

/// z-base-32 encoding, a [human oriented base-32 encoding](https://philzimmermann.com/docs/human-oriented-base-32-encoding.txt).
pub const Z: Encoding = Encoding::new(*b"ybndrfg8ejkmcpqxot1uwisza345h769");

/// Nix's Base32 encoding.
pub const NIX: Encoding = Encoding::new(*b"0123456789abcdfghijklmnpqrsvwxyz").with_reversed();

#[cfg(all(test, feature = "alloc"))]
mod test {
    use alloc::string::String;
    use alloc::vec::Vec;
    use core::fmt::Debug;
    use quickcheck::{Arbitrary, Gen};

    use super::Error;

    #[derive(Clone)]
    struct B32 {
        c: u8,
    }

    impl Arbitrary for B32 {
        fn arbitrary(g: &mut Gen) -> B32 {
            B32 {
                c: *g.choose(b"0123456789ABCDEFGHJKMNPQRSTVWXYZ").unwrap(),
            }
        }
    }

    impl Debug for B32 {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
            (self.c as char).fmt(f)
        }
    }

    #[test]
    fn masks_crockford() {
        assert_eq!(
            super::CROCKFORD.encode(&[0xF8, 0x3E, 0x0F, 0x83, 0xE0]),
            "Z0Z0Z0Z0"
        );
        assert_eq!(
            super::CROCKFORD.encode(&[0x07, 0xC1, 0xF0, 0x7C, 0x1F]),
            "0Z0Z0Z0Z"
        );
        assert_eq!(
            super::CROCKFORD.decode("Z0Z0Z0Z0").unwrap(),
            [0xF8, 0x3E, 0x0F, 0x83, 0xE0]
        );
        assert_eq!(
            super::CROCKFORD.decode("0Z0Z0Z0Z").unwrap(),
            [0x07, 0xC1, 0xF0, 0x7C, 0x1F]
        );
    }

    #[test]
    fn masks_rfc4648() {
        assert_eq!(
            super::RFC4648_NOPAD.encode(&[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "7A7H7A7H",
        );
        assert_eq!(
            super::RFC4648_NOPAD.encode(&[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "O7A7O7A7",
        );
        assert_eq!(
            super::RFC4648_NOPAD.decode("7A7H7A7H").unwrap(),
            [0xF8, 0x3E, 0x7F, 0x83, 0xE7],
        );
        assert_eq!(
            super::RFC4648_NOPAD.decode("O7A7O7A7").unwrap(),
            [0x77, 0xC1, 0xF7, 0x7C, 0x1F],
        );
        assert_eq!(
            super::RFC4648_NOPAD.encode(&[0xF8, 0x3E, 0x7F, 0x83]),
            "7A7H7AY",
        );
    }

    #[test]
    fn masks_rfc4648_pad() {
        assert_eq!(
            super::RFC4648.encode(&[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "7A7H7A7H",
        );
        assert_eq!(
            super::RFC4648.encode(&[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "O7A7O7A7",
        );
        assert_eq!(
            super::RFC4648.decode("7A7H7A7H").unwrap(),
            [0xF8, 0x3E, 0x7F, 0x83, 0xE7],
        );
        assert_eq!(
            super::RFC4648.decode("O7A7O7A7").unwrap(),
            [0x77, 0xC1, 0xF7, 0x7C, 0x1F],
        );
        assert_eq!(super::RFC4648.encode(&[0xF8, 0x3E, 0x7F, 0x83]), "7A7H7AY=");
    }

    #[test]
    fn masks_rfc4648_lower() {
        assert_eq!(
            super::RFC4648_LOWER_NOPAD.encode(&[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "7a7h7a7h",
        );
        assert_eq!(
            super::RFC4648_LOWER_NOPAD.encode(&[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "o7a7o7a7",
        );
        assert_eq!(
            super::RFC4648_LOWER_NOPAD.decode("7a7h7a7h").unwrap(),
            [0xF8, 0x3E, 0x7F, 0x83, 0xE7],
        );
        assert_eq!(
            super::RFC4648_LOWER_NOPAD.decode("o7a7o7a7").unwrap(),
            [0x77, 0xC1, 0xF7, 0x7C, 0x1F],
        );
        assert_eq!(
            super::RFC4648_LOWER_NOPAD.encode(&[0xF8, 0x3E, 0x7F, 0x83]),
            "7a7h7ay",
        );
    }

    #[test]
    fn masks_rfc4648_lower_pad() {
        assert_eq!(
            super::RFC4648_LOWER.encode(&[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "7a7h7a7h",
        );
        assert_eq!(
            super::RFC4648_LOWER.encode(&[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "o7a7o7a7",
        );
        assert_eq!(
            super::RFC4648_LOWER.decode("7a7h7a7h").unwrap(),
            [0xF8, 0x3E, 0x7F, 0x83, 0xE7],
        );
        assert_eq!(
            super::RFC4648_LOWER.decode("o7a7o7a7").unwrap(),
            [0x77, 0xC1, 0xF7, 0x7C, 0x1F],
        );
        assert_eq!(
            super::RFC4648_LOWER.encode(&[0xF8, 0x3E, 0x7F, 0x83]),
            "7a7h7ay=",
        );
    }

    #[test]
    fn masks_rfc4648_hex() {
        assert_eq!(
            super::RFC4648_HEX_NOPAD.encode(&[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "V0V7V0V7",
        );
        assert_eq!(
            super::RFC4648_HEX_NOPAD.encode(&[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "EV0VEV0V",
        );
        assert_eq!(
            super::RFC4648_HEX_NOPAD.decode("7A7H7A7H").unwrap(),
            [0x3A, 0x8F, 0x13, 0xA8, 0xF1],
        );
        assert_eq!(
            super::RFC4648_HEX_NOPAD.decode("O7A7O7A7").unwrap(),
            [0xC1, 0xD4, 0x7C, 0x1D, 0x47],
        );
        assert_eq!(
            super::RFC4648_HEX_NOPAD.encode(&[0xF8, 0x3E, 0x7F, 0x83]),
            "V0V7V0O",
        );
    }

    #[test]
    fn masks_rfc4648_hex_pad() {
        assert_eq!(
            super::RFC4648_HEX.encode(&[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "V0V7V0V7",
        );
        assert_eq!(
            super::RFC4648_HEX.encode(&[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "EV0VEV0V",
        );
        assert_eq!(
            super::RFC4648_HEX.decode("7A7H7A7H").unwrap(),
            [0x3A, 0x8F, 0x13, 0xA8, 0xF1],
        );
        assert_eq!(
            super::RFC4648_HEX.decode("O7A7O7A7").unwrap(),
            [0xC1, 0xD4, 0x7C, 0x1D, 0x47],
        );
        assert_eq!(
            super::RFC4648_HEX.encode(&[0xF8, 0x3E, 0x7F, 0x83]),
            "V0V7V0O=",
        );
    }

    #[test]
    fn masks_rfc4648_hex_lower() {
        assert_eq!(
            super::RFC4648_HEX_LOWER_NOPAD.encode(&[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "v0v7v0v7",
        );
        assert_eq!(
            super::RFC4648_HEX_LOWER_NOPAD.encode(&[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "ev0vev0v",
        );
        assert_eq!(
            super::RFC4648_HEX_LOWER_NOPAD.decode("7a7h7a7h").unwrap(),
            [0x3A, 0x8F, 0x13, 0xA8, 0xF1],
        );
        assert_eq!(
            super::RFC4648_HEX_LOWER_NOPAD.decode("o7a7o7a7").unwrap(),
            [0xC1, 0xD4, 0x7C, 0x1D, 0x47],
        );
        assert_eq!(
            super::RFC4648_HEX_LOWER_NOPAD.encode(&[0xF8, 0x3E, 0x7F, 0x83]),
            "v0v7v0o",
        );
    }

    #[test]
    fn masks_rfc4648_hex_lower_pad() {
        assert_eq!(
            super::RFC4648_HEX_LOWER.encode(&[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "v0v7v0v7",
        );
        assert_eq!(
            super::RFC4648_HEX_LOWER.encode(&[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "ev0vev0v",
        );
        assert_eq!(
            super::RFC4648_HEX_LOWER.decode("7a7h7a7h").unwrap(),
            [0x3A, 0x8F, 0x13, 0xA8, 0xF1],
        );
        assert_eq!(
            super::RFC4648_HEX_LOWER.decode("o7a7o7a7").unwrap(),
            [0xC1, 0xD4, 0x7C, 0x1D, 0x47],
        );
        assert_eq!(
            super::RFC4648_HEX_LOWER.encode(&[0xF8, 0x3E, 0x7F, 0x83]),
            "v0v7v0o=",
        );
    }

    #[test]
    fn masks_z() {
        assert_eq!(super::Z.encode(&[0xF8, 0x3E, 0x0F, 0x83, 0xE0]), "9y9y9y9y");
        assert_eq!(super::Z.encode(&[0x07, 0xC1, 0xF0, 0x7C, 0x1F]), "y9y9y9y9");
        assert_eq!(
            super::Z.decode("9y9y9y9y").unwrap(),
            [0xF8, 0x3E, 0x0F, 0x83, 0xE0],
        );
        assert_eq!(
            super::Z.decode("y9y9y9y9").unwrap(),
            [0x07, 0xC1, 0xF0, 0x7C, 0x1F]
        );
    }

    #[test]
    fn padding() {
        let num_padding = [0, 6, 4, 3, 1];
        for i in 1..6 {
            let encoded = super::RFC4648.encode((0..(i as u8)).collect::<Vec<u8>>().as_ref());
            assert_eq!(encoded.len(), 8);
            for j in 0..(num_padding[i % 5]) {
                assert_eq!(encoded.as_bytes()[encoded.len() - j - 1], b'=');
            }
            for j in 0..(8 - num_padding[i % 5]) {
                assert!(encoded.as_bytes()[j] != b'=');
            }
        }
    }

    #[test]
    fn invertible_crockford() {
        fn test(data: Vec<u8>) -> bool {
            super::CROCKFORD
                .decode(&super::CROCKFORD.encode(&data))
                .unwrap()
                == data
        }
        quickcheck::quickcheck(test as fn(Vec<u8>) -> bool)
    }

    #[test]
    fn invertible_rfc4648() {
        fn test(data: Vec<u8>) -> bool {
            super::RFC4648
                .decode(&super::RFC4648.encode(&data))
                .unwrap()
                == data
        }
        quickcheck::quickcheck(test as fn(Vec<u8>) -> bool)
    }
    #[test]
    fn invertible_unpadded_rfc4648() {
        fn test(data: Vec<u8>) -> bool {
            super::RFC4648_NOPAD
                .decode(&super::RFC4648_NOPAD.encode(&data))
                .unwrap()
                == data
        }
        quickcheck::quickcheck(test as fn(Vec<u8>) -> bool)
    }

    #[test]
    fn lower_case() {
        fn test(data: Vec<B32>) -> bool {
            let data: String = data.iter().map(|e| e.c as char).collect();
            super::CROCKFORD.decode(&data.as_ref())
                == super::CROCKFORD.decode(data.to_ascii_lowercase().as_ref())
        }
        quickcheck::quickcheck(test as fn(Vec<B32>) -> bool)
    }

    #[test]
    #[allow(non_snake_case)]
    fn iIlL1_oO0() {
        assert_eq!(
            super::CROCKFORD.decode("IiLl1Oo0").unwrap(),
            super::CROCKFORD.decode("11111000").unwrap(),
        );
    }

    #[test]
    fn invalid_chars_crockford() {
        assert_eq!(
            super::CROCKFORD.decode(",."),
            Err(Error::InvalidSymbol {
                offset: 0,
                symbol: b','
            }),
        )
    }

    #[test]
    fn invalid_chars_rfc4648() {
        assert_eq!(
            super::RFC4648.decode(",.======"),
            Err(Error::InvalidSymbol {
                offset: 0,
                symbol: b','
            }),
        )
    }

    #[test]
    fn invalid_chars_unpadded_rfc4648() {
        assert_eq!(
            super::RFC4648_NOPAD.decode(",."),
            Err(Error::InvalidSymbol {
                offset: 0,
                symbol: b','
            }),
        )
    }

    #[test]
    fn nix() {
        // Copied from https://nix.dev/manual/nix/2.28/command-ref/nix-hash#examples
        let unencoded = [
            0xE4, 0xFD, 0x8B, 0xA5, 0xF7, 0xBB, 0xEA, 0xEA, 0x5A, 0xCE, 0x89, 0xFE, 0x10, 0x25,
            0x55, 0x36, 0xCD, 0x60, 0xDA, 0xB6,
        ];
        let encoded = "nvd61k9nalji1zl9rrdfmsmvyyjqpzg4";

        assert_eq!(super::NIX.encode(&unencoded), encoded);
        assert_eq!(super::NIX.decode(&encoded).unwrap(), unencoded);
    }
}

#[cfg(all(doctest, feature = "alloc"))]
#[doc = include_str!("../README.md")]
struct Readme;
