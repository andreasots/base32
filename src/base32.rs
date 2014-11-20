#![crate_name="base32"]
#![crate_type="rlib"]

#![feature(phase)]

#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[phase(plugin)]
extern crate quickcheck_macros;

use std::iter::range_inclusive;
use std::cmp::min;

use Base32Type::{RFC4648Base32, CrockfordBase32, UnpaddedRFC4648Base32};

pub enum Base32Type {
    // Not sure if it's better to just have an additional `padding: bool`
    // argument to the encode function
    RFC4648Base32, CrockfordBase32, UnpaddedRFC4648Base32
}

static RFC4648_ALPHABET: &'static str   = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
static CROCKFORD_ALPHABET: &'static str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

pub fn encode(base32_type: Base32Type, data: &[u8]) -> Vec<Ascii> {
    let alphabet = match base32_type {
        RFC4648Base32 | UnpaddedRFC4648Base32 => RFC4648_ALPHABET,
        CrockfordBase32 => CROCKFORD_ALPHABET
    }.to_ascii();
    let mut ret = Vec::with_capacity((data.len()+3)/4*5);

    for chunk in data.chunks(5) {
        let buf = {
            let mut buf = [0u8, ..5];
            buf.clone_from_slice(chunk);
            buf
        };
        ret.push(alphabet[((buf[0] & 0xF8) >> 3) as uint]);
        ret.push(alphabet[(((buf[0] & 0x07) << 2) | ((buf[1] & 0xC0) >> 6)) as uint]);
        ret.push(alphabet[((buf[1] & 0x3E) >> 1) as uint]);
        ret.push(alphabet[(((buf[1] & 0x01) << 4) | ((buf[2] & 0xF0) >> 4)) as uint]);
        ret.push(alphabet[(((buf[2] & 0x0F) << 1) | (buf[3] >> 7)) as uint]);
        ret.push(alphabet[((buf[3] & 0x7C) >> 2) as uint]);
        ret.push(alphabet[(((buf[3] & 0x03) << 3) | ((buf[4] & 0xE0) >> 5)) as uint]);
        ret.push(alphabet[(buf[4] & 0x1F) as uint]);
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
                    ret[len-i] = '='.to_ascii();
                }
            }
        }
    }

    ret
}

static RFC4648_INV_ALPHABET: [u8, ..43] = [-1, -1, 26, 27, 28, 29, 30, 31, -1, -1, -1, -1, -1, 0, -1, -1, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25];

static CROCKFORD_INV_ALPHABET: [u8, ..43] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, -1, -1, -1, -1, -1, -1, -1, 10, 11, 12, 13, 14, 15, 16, 17, 1, 18, 19, 1, 20, 21, 0, 22, 23, 24, 25, 26, -1, 27, 28, 29, 30, 31];

pub fn decode(base32_type: Base32Type, data: &[Ascii]) -> Option<Vec<u8>> {
    let alphabet = match base32_type {
        RFC4648Base32 | UnpaddedRFC4648Base32 => RFC4648_INV_ALPHABET,
        CrockfordBase32 => CROCKFORD_INV_ALPHABET
    };
    let mut unpadded_data_length = data.len();
    for i in range_inclusive(1u, min(6, data.len())) {
        if data[data.len() - i] != '='.to_ascii() {
            break;
        }
        unpadded_data_length -= 1;
    }
    let output_length = unpadded_data_length*5/8;
    let mut ret = Vec::with_capacity((output_length+4)/5*5);
    for chunk in data.chunks(8) {
        let buf = {
            let mut buf = [0u8, ..8];
            for (i, &c) in chunk.iter().enumerate() {
                match alphabet.get((c.to_uppercase().to_byte()-('0' as u8)) as uint) {
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
    use std::rand::distributions::IndependentSample;

    #[deriving(Clone)]
    struct B32 {
        c: Ascii
    }

    impl quickcheck::Arbitrary for B32 {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> B32 {
            let alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ".to_ascii();
            B32 {
                c: alphabet[std::rand::distributions::Range::new(0, alphabet.len()).ind_sample(g)]
            }
        }
    }

    impl std::fmt::Show for B32 {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
            self.c.fmt(f)
        }
    }

    #[test]
    fn masks_crockford() {
        assert_eq!(&*encode(CrockfordBase32, &[0xF8, 0x3E, 0x0F, 0x83, 0xE0]), "Z0Z0Z0Z0".to_ascii());
        assert_eq!(&*encode(CrockfordBase32, &[0x07, 0xC1, 0xF0, 0x7C, 0x1F]), "0Z0Z0Z0Z".to_ascii());
        assert_eq!(&*decode(CrockfordBase32, "Z0Z0Z0Z0".to_ascii()).unwrap(), [0xF8, 0x3E, 0x0F, 0x83, 0xE0].as_slice());
        assert_eq!(&*decode(CrockfordBase32, "0Z0Z0Z0Z".to_ascii()).unwrap(), [0x07, 0xC1, 0xF0, 0x7C, 0x1F].as_slice());
    }

    #[test]
    fn masks_rfc4648() {
        assert_eq!(&*encode(RFC4648Base32, &[0xF8, 0x3E, 0x7F, 0x83, 0xE7]), "7A7H7A7H".to_ascii());
        assert_eq!(&*encode(RFC4648Base32, &[0x77, 0xC1, 0xF7, 0x7C, 0x1F]), "O7A7O7A7".to_ascii());
        assert_eq!(&*decode(RFC4648Base32, "7A7H7A7H".to_ascii()).unwrap(), [0xF8, 0x3E, 0x7F, 0x83, 0xE7].as_slice());
        assert_eq!(&*decode(RFC4648Base32, "O7A7O7A7".to_ascii()).unwrap(), [0x77, 0xC1, 0xF7, 0x7C, 0x1F].as_slice());
        assert_eq!(&*encode(RFC4648Base32, &[0xF8, 0x3E, 0x7F, 0x83]), "7A7H7AY=".to_ascii());
    }

    #[test]
    fn masks_unpadded_rfc4648() {
        assert_eq!(&*encode(UnpaddedRFC4648Base32, &[0xF8, 0x3E, 0x7F, 0x83, 0xE7]), "7A7H7A7H".to_ascii());
        assert_eq!(&*encode(UnpaddedRFC4648Base32, &[0x77, 0xC1, 0xF7, 0x7C, 0x1F]), "O7A7O7A7".to_ascii());
        assert_eq!(&*decode(UnpaddedRFC4648Base32, "7A7H7A7H".to_ascii()).unwrap(), [0xF8, 0x3E, 0x7F, 0x83, 0xE7].as_slice());
        assert_eq!(&*decode(UnpaddedRFC4648Base32, "O7A7O7A7".to_ascii()).unwrap(), [0x77, 0xC1, 0xF7, 0x7C, 0x1F].as_slice());
        assert_eq!(&*encode(UnpaddedRFC4648Base32, &[0xF8, 0x3E, 0x7F, 0x83]), "7A7H7AY".to_ascii());
    }

    #[test]
    fn padding() {
        let num_padding = [0u, 6, 4, 3, 1];
        for i in range(1u, 6) {
            println!("Checking padding for length == {}", i);
            let encoded = encode(RFC4648Base32, range(0u8, i as u8).collect::<Vec<u8>>().as_slice());
            assert_eq!(encoded.len(), 8u);
            for j in range(0u, num_padding[i % 5]) {
                println!("Making sure index {} is padding", encoded.len()-j-1);
                assert_eq!(encoded[encoded.len()-j-1], '='.to_ascii());
            }
            for j in range(0u, 8 - num_padding[i % 5]) {
                println!("Making sure index {} is not padding", j);
                assert!(encoded[j] != '='.to_ascii());
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
        let data: Vec<Ascii> = data.iter().map(|e| e.c).collect();
        decode(CrockfordBase32, data.as_slice()) == decode(CrockfordBase32, data.as_slice().to_lowercase().as_slice())
    }

    #[test]
    #[allow(non_snake_case)]
    fn iIlL1_oO0() {
        assert_eq!(decode(CrockfordBase32, "IiLlOo".to_ascii()), decode(CrockfordBase32, "111100".to_ascii()));
    }

    #[test]
    fn invalid_chars_crockford() {
        assert_eq!(decode(CrockfordBase32, ",".to_ascii()), None)
    }

    #[test]
    fn invalid_chars_rfc4648() {
        assert_eq!(decode(RFC4648Base32, ",".to_ascii()), None)
    }

    #[test]
    fn invalid_chars_unpadded_rfc4648() {
        assert_eq!(decode(UnpaddedRFC4648Base32, ",".to_ascii()), None)
    }

    #[bench]
    fn bench_encode(b: &mut test::Bencher) {
        let data = [0, 0, 0, 0, 0];
        b.iter(|| encode(CrockfordBase32, data.as_slice()));
        b.bytes = data.len() as u64;
    }

    #[bench]
    fn bench_decode(b: &mut test::Bencher) {
        let data = "00000000".to_ascii();
        b.iter(|| decode(CrockfordBase32, data));
        b.bytes = data.len() as u64;
    }
}
