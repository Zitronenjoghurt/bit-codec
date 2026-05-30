use super::*;

mod derive;

pub fn encode_to_vec<T: BitEncode>(v: &T) -> Vec<u8> {
    let mut w = BitWriter::new(Vec::new());
    v.encode(&mut w).unwrap();
    w.flush().unwrap();
    w.into_inner()
}

pub fn decode_from_bytes<T: BitDecode>(bytes: &[u8]) -> std::io::Result<T> {
    let mut r = BitReader::new(bytes);
    T::decode(&mut r)
}

pub fn roundtrip<T: BitEncode + BitDecode>(v: &T) -> T {
    decode_from_bytes(&encode_to_vec(v)).unwrap()
}

#[test]
fn round_trip_integers() {
    let mut buf = Vec::new();
    {
        let mut w = BitWriter::new(&mut buf);
        w.write(&42u8).unwrap();
        w.write(&0xABCDu16).unwrap();
        w.write(&0xDEAD_BEEFu32).unwrap();
        w.write(&0x0102_0304_0506_0708u64).unwrap();
        w.write(&-1i32).unwrap();
        w.flush().unwrap();
    }

    let mut r = BitReader::new(buf.as_slice());
    assert_eq!(r.read::<u8>().unwrap(), 42);
    assert_eq!(r.read::<u16>().unwrap(), 0xABCD);
    assert_eq!(r.read::<u32>().unwrap(), 0xDEAD_BEEF);
    assert_eq!(r.read::<u64>().unwrap(), 0x0102_0304_0506_0708);
    assert_eq!(r.read::<i32>().unwrap(), -1);
}

#[test]
fn round_trip_bool() {
    let mut buf = Vec::new();
    {
        let mut w = BitWriter::new(&mut buf);
        w.write(&true).unwrap();
        w.write(&false).unwrap();
        w.write(&true).unwrap();
        w.flush().unwrap();
    }

    let mut r = BitReader::new(buf.as_slice());
    assert!(r.read::<bool>().unwrap());
    assert!(!r.read::<bool>().unwrap());
    assert!(r.read::<bool>().unwrap());
}

#[test]
fn round_trip_string() {
    let mut buf = Vec::new();
    {
        let mut w = BitWriter::new(&mut buf);
        w.write(&String::from("hello")).unwrap();
        w.write(&"world").unwrap();
        w.flush().unwrap();
    }

    let mut r = BitReader::new(buf.as_slice());
    assert_eq!(r.read::<String>().unwrap(), "hello");
    assert_eq!(r.read::<String>().unwrap(), "world");
}

#[test]
fn round_trip_vec() {
    let mut buf = Vec::new();
    {
        let mut w = BitWriter::new(&mut buf);
        w.write(&vec![1u32, 2, 3, 4, 5]).unwrap();
        w.write(&vec!["foo".to_string(), "bar".to_string()])
            .unwrap();
        w.flush().unwrap();
    }

    let mut r = BitReader::new(buf.as_slice());
    assert_eq!(r.read::<Vec<u32>>().unwrap(), vec![1, 2, 3, 4, 5]);
    assert_eq!(
        r.read::<Vec<String>>().unwrap(),
        vec!["foo".to_string(), "bar".to_string()]
    );
}

#[test]
fn round_trip_mixed_bits() {
    let mut buf = Vec::new();
    {
        let mut w = BitWriter::new(&mut buf);
        w.write_bits(0b110u32, 3).unwrap();
        w.write(&true).unwrap();
        w.write(&0xFFu8).unwrap();
        w.write(&1024u16).unwrap();
        w.flush().unwrap();
    }

    let mut r = BitReader::new(buf.as_slice());
    assert_eq!(r.read_bits::<u8>(3).unwrap(), 0b110);
    assert!(r.read::<bool>().unwrap());
    assert_eq!(r.read::<u8>().unwrap(), 0xFF);
    assert_eq!(r.read::<u16>().unwrap(), 1024);
}

#[test]
fn nibbles() {
    let mut buf = Vec::new();
    {
        let mut w = BitWriter::new(&mut buf);
        w.write_bits(0xAu32, 4).unwrap();
        w.write_bits(0x5u32, 4).unwrap();
        w.flush().unwrap();
    }
    assert_eq!(buf, [0xA5]);

    let mut r = BitReader::new(buf.as_slice());
    assert_eq!(r.read_bits::<u8>(4).unwrap(), 0xA);
    assert_eq!(r.read_bits::<u8>(4).unwrap(), 0x5);
}

#[test]
fn byte_level() {
    let original = b"Hello, bits!";
    let mut buf = Vec::new();
    {
        let mut w = BitWriter::new(&mut buf);
        w.write_full_bytes(original).unwrap();
        w.flush().unwrap();
    }

    let mut r = BitReader::new(buf.as_slice());
    let mut out = vec![0u8; original.len()];
    r.read_full_bytes(&mut out).unwrap();
    assert_eq!(&out, original);
}

#[test]
fn alignment() {
    let mut buf = Vec::new();
    {
        let mut w = BitWriter::new(&mut buf);
        w.write_bits(0b111u8, 3).unwrap();
        w.align().unwrap();
        w.write(&0xABu8).unwrap();
        w.flush().unwrap();
    }
    assert_eq!(buf, [0b1110_0000, 0xAB]);
}

#[test]
fn std_read_trait() {
    let data: &[u8] = &[0x01, 0x02, 0x03, 0x04];
    let mut r = BitReader::new(data);
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf).unwrap();
    assert_eq!(buf, [0x01, 0x02, 0x03, 0x04]);
}

#[test]
fn std_write_trait() {
    let mut buf = Vec::new();
    {
        let mut w = BitWriter::new(&mut buf);
        w.write_all(b"raw bytes").unwrap();
        w.flush().unwrap();
    }
    assert_eq!(&buf, b"raw bytes");
}

#[test]
fn eof_error() {
    let data: &[u8] = &[0xFF];
    let mut r = BitReader::new(data);
    r.read::<u8>().unwrap();
    assert!(r.read_bits::<u8>(1).is_err());
}

#[test]
fn round_trip_floats() {
    let mut buf = Vec::new();
    {
        let mut w = BitWriter::new(&mut buf);
        w.write(&std::f32::consts::PI).unwrap();
        w.write(&std::f64::consts::E).unwrap();
        w.flush().unwrap();
    }

    let mut r = BitReader::new(buf.as_slice());
    assert_eq!(r.read::<f32>().unwrap(), std::f32::consts::PI);
    assert_eq!(r.read::<f64>().unwrap(), std::f64::consts::E);
}

#[test]
fn round_trip_slice() {
    let mut buf = Vec::new();
    {
        let mut w = BitWriter::new(&mut buf);
        let data: &[u16] = &[10, 20, 30];
        w.write(data).unwrap();
        w.flush().unwrap();
    }

    let mut r = BitReader::new(buf.as_slice());
    assert_eq!(r.read::<Vec<u16>>().unwrap(), vec![10, 20, 30]);
}

#[test]
fn primitive_roundtrips() {
    assert_eq!(roundtrip(&42u8), 42u8);
    assert_eq!(roundtrip(&1234u16), 1234u16);
    assert_eq!(roundtrip(&u32::MAX), u32::MAX);
    assert_eq!(roundtrip(&-1i32), -1i32);
    assert_eq!(roundtrip(&std::f64::consts::PI), std::f64::consts::PI);
}

#[test]
fn bool_roundtrip() {
    assert_eq!(roundtrip(&true), true);
    assert_eq!(roundtrip(&false), false);
}

#[test]
fn option_some_none() {
    let some_val: Option<u16> = Some(999);
    let none_val: Option<u16> = None;
    assert_eq!(roundtrip(&some_val), Some(999));
    assert_eq!(roundtrip(&none_val), None);
}

#[test]
fn string_roundtrip() {
    assert_eq!(roundtrip(&String::from("hello")), "hello");
    assert_eq!(roundtrip(&String::new()), "");
}

#[test]
fn string_decode_invalid_utf8() {
    let mut w = BitWriter::new(Vec::new());
    VarLen(2).encode(&mut w).unwrap();
    w.write_full_bytes(&[0xFF, 0xFE]).unwrap();
    w.flush().unwrap();
    let bytes = w.into_inner();
    let result = decode_from_bytes::<String>(&bytes);
    assert!(result.is_err(), "should reject invalid UTF-8");
}

#[test]
fn vec_roundtrip() {
    let v: Vec<u16> = vec![1, 2, 3, 4, 5];
    assert_eq!(roundtrip(&v), v);
}

#[test]
fn vec_empty() {
    let v: Vec<u32> = vec![];
    assert_eq!(roundtrip(&v), v);
}

#[test]
fn vec_large_length_uses_varlen() {
    let v: Vec<u8> = vec![0xAB; 300];
    assert_eq!(roundtrip(&v), v);
}

#[test]
fn mixed_sub_byte_fields() {
    let mut w = BitWriter::new(Vec::new());
    w.write_bits(0b101u8, 3).unwrap();
    w.write_bits(1u8, 1).unwrap();
    w.write_bits(0b1100u8, 4).unwrap();
    w.flush().unwrap();
    let bytes = w.into_inner();
    assert_eq!(bytes.len(), 1);

    let mut r = BitReader::new(&bytes[..]);
    assert_eq!(r.read_bits::<u8>(3).unwrap(), 0b101);
    assert_eq!(r.read_bits::<u8>(1).unwrap(), 1);
    assert_eq!(r.read_bits::<u8>(4).unwrap(), 0b1100);
}

#[test]
fn write_full_bytes_unaligned_then_read_back() {
    let mut w = BitWriter::new(Vec::new());
    w.write_bits(0b11u8, 2).unwrap();
    w.write_full_bytes(&[0xDE, 0xAD]).unwrap();
    w.write_bits(0b1010u8, 4).unwrap();
    w.flush().unwrap();
    let bytes = w.into_inner();

    let mut r = BitReader::new(&bytes[..]);
    assert_eq!(r.read_bits::<u8>(2).unwrap(), 0b11);
    let mut buf = [0u8; 2];
    r.read_full_bytes(&mut buf).unwrap();
    assert_eq!(buf, [0xDE, 0xAD]);
    assert_eq!(r.read_bits::<u8>(4).unwrap(), 0b1010);
}
