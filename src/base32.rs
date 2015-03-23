#![feature(core, collections, custom_attribute)]

#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use]
#[no_link]
extern crate quickcheck_macros;
#[cfg(test)]
extern crate rand;

use std::iter::range_inclusive;
use std::cmp::min;

use Base32Type::{RFC4648Base32, CrockfordBase32, UnpaddedRFC4648Base32};

use std::ascii::AsciiExt;

#[derive(Copy)]
pub enum Base32Type {
    // Not sure if it's better to just have an additional `padding: bool`
    // argument to the encode function
    RFC4648Base32, CrockfordBase32, UnpaddedRFC4648Base32
}

const RFC4648_ALPHABET: &'static [u8]   = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
const CROCKFORD_ALPHABET: &'static [u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

pub fn encode(base32_type: Base32Type, data: &[u8]) -> String {
    let alphabet = match base32_type {
        RFC4648Base32 | UnpaddedRFC4648Base32 => RFC4648_ALPHABET,
        CrockfordBase32 => CROCKFORD_ALPHABET
    };
    let mut ret = Vec::with_capacity((data.len()+3)/4*5);

    for chunk in data.chunks(5) {
        let buf = {
            let mut buf = [0u8; 5];
            buf.clone_from_slice(chunk);
            buf
        };
        ret.push(alphabet[((buf[0] & 0xF8) >> 3) as usize]);
        ret.push(alphabet[(((buf[0] & 0x07) << 2) | ((buf[1] & 0xC0) >> 6)) as usize]);
        ret.push(alphabet[((buf[1] & 0x3E) >> 1) as usize]);
        ret.push(alphabet[(((buf[1] & 0x01) << 4) | ((buf[2] & 0xF0) >> 4)) as usize]);
        ret.push(alphabet[(((buf[2] & 0x0F) << 1) | (buf[3] >> 7)) as usize]);
        ret.push(alphabet[((buf[3] & 0x7C) >> 2) as usize]);
        ret.push(alphabet[(((buf[3] & 0x03) << 3) | ((buf[4] & 0xE0) >> 5)) as usize]);
        ret.push(alphabet[(buf[4] & 0x1F) as usize]);
    }

    if data.len() % 5 != 0 {
        let len = ret.len();
        let num_extra = 8-(data.len()%5*8+4)/5;
        match base32_type {
            UnpaddedRFC4648Base32 | CrockfordBase32 => {
                ret.truncate(len-num_extra);
            }
            RFC4648Base32 => {
                for i in range_inclusive(1, num_extra) {
                    ret[len-i] = b'=';
                }
            }
        }
    }

    String::from_utf8(ret).unwrap()
}

const RFC4648_INV_ALPHABET: [u8; 43] = [-1, -1, 26, 27, 28, 29, 30, 31, -1, -1, -1, -1, -1, 0, -1, -1, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25];

const CROCKFORD_INV_ALPHABET: [u8; 43] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, -1, -1, -1, -1, -1, -1, -1, 10, 11, 12, 13, 14, 15, 16, 17, 1, 18, 19, 1, 20, 21, 0, 22, 23, 24, 25, 26, -1, 27, 28, 29, 30, 31];

pub fn decode(base32_type: Base32Type, data: &str) -> Option<Vec<u8>> {
    if !data.is_ascii() {
        return None;
    }
    let data = data.as_bytes();
    let alphabet = match base32_type {
        RFC4648Base32 | UnpaddedRFC4648Base32 => RFC4648_INV_ALPHABET,
        CrockfordBase32 => CROCKFORD_INV_ALPHABET
    };
    let mut unpadded_data_length = data.len();
    for i in range_inclusive(1, min(6, data.len())) {
        if data[data.len() - i] != b'=' {
            break;
        }
        unpadded_data_length -= 1;
    }
    let output_length = unpadded_data_length*5/8;
    let mut ret = Vec::with_capacity((output_length+4)/5*5);
    for chunk in data.chunks(8) {
        let buf = {
            let mut buf = [0u8; 8];
            for (i, &c) in chunk.iter().enumerate() {
                match alphabet.get(c.to_ascii_uppercase().wrapping_sub(b'0') as usize) {
                    Some(&-1) | None => return None,
                    Some(&value) => buf[i] = value,
                };
            }
            buf
        };
        ret.push((buf[0] << 3) | (buf[1] >> 2));
        ret.push((buf[1] << 6) | (buf[2] << 1) | (buf[3] >> 4));
        ret.push((buf[3] << 4) | (buf[4] >> 1));
        ret.push((buf[4] << 7) | (buf[5] << 2) | (buf[6] >> 3));
        ret.push((buf[6] << 5) | buf[7]);
    }
    ret.truncate(output_length);
    Some(ret)
}

#[cfg(test)]
mod test {
    extern crate test;
    use super::{encode, decode};
    use super::Base32Type::{CrockfordBase32, RFC4648Base32, UnpaddedRFC4648Base32};
    use quickcheck;
    use std;
    use std::ascii::AsciiExt;
    use rand::distributions::{IndependentSample, Range};

    #[derive(Clone)]
    struct B32 {
        c: u8
    }

    impl quickcheck::Arbitrary for B32 {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> B32 {
            let alphabet = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
            B32 {
                c: alphabet[Range::new(0, alphabet.len()).ind_sample(g)]
            }
        }
    }

    impl std::fmt::Display for B32 {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
            self.c.fmt(f)
        }
    }

    #[test]
    fn masks_crockford() {
        assert_eq!(encode(CrockfordBase32, &[0xF8, 0x3E, 0x0F, 0x83, 0xE0]), "Z0Z0Z0Z0");
        assert_eq!(encode(CrockfordBase32, &[0x07, 0xC1, 0xF0, 0x7C, 0x1F]), "0Z0Z0Z0Z");
        assert_eq!(decode(CrockfordBase32, "Z0Z0Z0Z0").unwrap(), [0xF8, 0x3E, 0x0F, 0x83, 0xE0]);
        assert_eq!(decode(CrockfordBase32, "0Z0Z0Z0Z").unwrap(), [0x07, 0xC1, 0xF0, 0x7C, 0x1F]);
    }

    #[test]
    fn masks_rfc4648() {
        assert_eq!(encode(RFC4648Base32, &[0xF8, 0x3E, 0x7F, 0x83, 0xE7]), "7A7H7A7H");
        assert_eq!(encode(RFC4648Base32, &[0x77, 0xC1, 0xF7, 0x7C, 0x1F]), "O7A7O7A7");
        assert_eq!(decode(RFC4648Base32, "7A7H7A7H").unwrap(), [0xF8, 0x3E, 0x7F, 0x83, 0xE7]);
        assert_eq!(decode(RFC4648Base32, "O7A7O7A7").unwrap(), [0x77, 0xC1, 0xF7, 0x7C, 0x1F]);
        assert_eq!(encode(RFC4648Base32, &[0xF8, 0x3E, 0x7F, 0x83]), "7A7H7AY=");
    }

    #[test]
    fn masks_unpadded_rfc4648() {
        assert_eq!(encode(UnpaddedRFC4648Base32, &[0xF8, 0x3E, 0x7F, 0x83, 0xE7]), "7A7H7A7H");
        assert_eq!(encode(UnpaddedRFC4648Base32, &[0x77, 0xC1, 0xF7, 0x7C, 0x1F]), "O7A7O7A7");
        assert_eq!(decode(UnpaddedRFC4648Base32, "7A7H7A7H").unwrap(), [0xF8, 0x3E, 0x7F, 0x83, 0xE7]);
        assert_eq!(decode(UnpaddedRFC4648Base32, "O7A7O7A7").unwrap(), [0x77, 0xC1, 0xF7, 0x7C, 0x1F]);
        assert_eq!(encode(UnpaddedRFC4648Base32, &[0xF8, 0x3E, 0x7F, 0x83]), "7A7H7AY");
    }

    #[test]
    fn padding() {
        let num_padding = [0, 6, 4, 3, 1];
        for i in 1..6 {
            println!("Checking padding for length == {}", i);
            let encoded = encode(RFC4648Base32, (0..(i as u8)).collect::<Vec<u8>>().as_slice());
            assert_eq!(encoded.len(), 8);
            for j in 0..(num_padding[i % 5]) {
                println!("Making sure index {} is padding", encoded.len()-j-1);
                assert_eq!(encoded.as_bytes()[encoded.len()-j-1], b'=');
            }
            for j in 0..(8 - num_padding[i % 5]) {
                println!("Making sure index {} is not padding", j);
                assert!(encoded.as_bytes()[j] != b'=');
            }
        }
    }

    #[quickcheck]
    fn invertible_crockford(data: Vec<u8>) -> bool {
        decode(CrockfordBase32, encode(CrockfordBase32, data.as_slice()).as_slice()).unwrap() == data
    }

    #[quickcheck]
    fn invertible_rfc4648(data: Vec<u8>) -> bool {
        decode(RFC4648Base32, encode(RFC4648Base32, data.as_slice()).as_slice()).unwrap() == data
    }
    #[quickcheck]
    fn invertible_unpadded_rfc4648(data: Vec<u8>) -> bool {
        decode(UnpaddedRFC4648Base32, encode(UnpaddedRFC4648Base32, data.as_slice()).as_slice()).unwrap() == data
    }

    #[quickcheck]
    fn lower_case(data: Vec<B32>) -> bool {
        let data: String = data.iter().map(|e| e.c as char).collect();
        decode(CrockfordBase32, data.as_slice()) == decode(CrockfordBase32, data.as_slice().to_ascii_lowercase().as_slice())
    }

    #[test]
    #[allow(non_snake_case)]
    fn iIlL1_oO0() {
        assert_eq!(decode(CrockfordBase32, "IiLlOo"), decode(CrockfordBase32, "111100"));
    }

    #[test]
    fn invalid_chars_crockford() {
        assert_eq!(decode(CrockfordBase32, ","), None)
    }

    #[test]
    fn invalid_chars_rfc4648() {
        assert_eq!(decode(RFC4648Base32, ","), None)
    }

    #[test]
    fn invalid_chars_unpadded_rfc4648() {
        assert_eq!(decode(UnpaddedRFC4648Base32, ","), None)
    }

    #[bench]
    fn bench_encode(b: &mut test::Bencher) {
        let data = [0, 0, 0, 0, 0];
        b.iter(|| encode(CrockfordBase32, data.as_slice()));
        b.bytes = data.len() as u64;
    }

    #[bench]
    fn bench_decode(b: &mut test::Bencher) {
        let data = "00000000";
        b.iter(|| decode(CrockfordBase32, data));
        b.bytes = data.len() as u64;
    }
}
