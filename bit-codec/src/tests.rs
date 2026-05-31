use super::*;

mod derive;

pub fn encode_to_vec<T: BitEncode>(v: &T) -> Vec<u8> {
    let mut w = BitWriter::new(Vec::new());
    v.encode(&mut w).unwrap();
    w.flush().unwrap();
    w.into_inner()
}

pub fn decode_from_bytes<T: BitDecode>(bytes: &[u8]) -> io::Result<T> {
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
        w.write(&4782174usize).unwrap();
        w.write(&-447742isize).unwrap();
        w.flush().unwrap();
    }

    let mut r = BitReader::new(buf.as_slice());
    assert_eq!(r.read_bits::<u8>(3).unwrap(), 0b110);
    assert!(r.read::<bool>().unwrap());
    assert_eq!(r.read::<u8>().unwrap(), 0xFF);
    assert_eq!(r.read::<u16>().unwrap(), 1024);
    assert_eq!(r.read::<usize>().unwrap(), 4782174);
    assert_eq!(r.read::<isize>().unwrap(), -447742);
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
    assert!(roundtrip(&true));
    assert!(!roundtrip(&false));
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

#[test]
fn array_roundtrip() {
    assert_eq!(roundtrip(&[1u8, 2, 3, 4, 5]), [1u8, 2, 3, 4, 5]);
    assert_eq!(roundtrip(&[0xDEADu16, 0xBEEF]), [0xDEADu16, 0xBEEF]);
    assert_eq!(roundtrip(&[true, false, true]), [true, false, true]);
}

#[test]
fn array_empty() {
    assert_eq!(roundtrip(&([] as [u32; 0])), [] as [u32; 0]);
}

#[test]
fn array_nested_in_vec() {
    let v: Vec<[u8; 3]> = vec![[1, 2, 3], [4, 5, 6]];
    assert_eq!(roundtrip(&v), v);
}

#[test]
fn array_no_length_prefix() {
    let arr = [1u8, 2u8];
    let vec = vec![1u8, 2u8];
    let arr_bytes = encode_to_vec(&arr);
    let vec_bytes = encode_to_vec(&vec);
    assert!(
        arr_bytes.len() < vec_bytes.len(),
        "array should be smaller than vec (no length prefix)"
    );
    assert_eq!(arr_bytes, [1, 2]);
}

#[test]
fn unit_roundtrip() {
    assert_eq!(roundtrip(&()), ());
}

#[test]
fn unit_zero_bytes() {
    assert!(encode_to_vec(&()).is_empty());
}

#[test]
fn tuple_1() {
    assert_eq!(roundtrip(&(42u8,)), (42u8,));
}

#[test]
fn tuple_2_heterogeneous() {
    assert_eq!(
        roundtrip(&(true, "hi".to_string())),
        (true, "hi".to_string())
    );
}

#[test]
fn tuple_3() {
    assert_eq!(roundtrip(&(1u8, 2u16, 3u32)), (1u8, 2u16, 3u32));
}

#[test]
fn tuple_4() {
    assert_eq!(
        roundtrip(&(false, -1i32, "x".to_string(), std::f64::consts::PI)),
        (false, -1i32, "x".to_string(), std::f64::consts::PI)
    );
}

#[test]
fn tuple_12() {
    let val = (
        1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8, 11u8, 12u8,
    );
    assert_eq!(roundtrip(&val), val);
}

#[test]
fn tuple_nested() {
    let val = ((1u8, 2u16), (true, "nested".to_string()));
    assert_eq!(roundtrip(&val), val);
}

#[test]
fn box_roundtrip() {
    assert_eq!(roundtrip(&Box::new(42u32)), Box::new(42u32));
}

#[test]
fn box_str_roundtrip() {
    let val: Box<str> = "hello".into();
    assert_eq!(roundtrip(&val), val);
}

#[test]
fn box_slice_roundtrip() {
    let val: Box<[u16]> = vec![1, 2, 3].into_boxed_slice();
    assert_eq!(roundtrip(&val), val);
}

#[test]
fn box_nested() {
    assert_eq!(
        roundtrip(&Box::new(Box::new(99u8))),
        Box::new(Box::new(99u8))
    );
}

#[test]
fn cow_str_borrowed() {
    let val: Cow<str> = Cow::Borrowed("borrowed");
    assert_eq!(&*roundtrip(&val), "borrowed");
}

#[test]
fn cow_str_owned() {
    let val: Cow<str> = Cow::Owned("owned".to_string());
    assert_eq!(&*roundtrip(&val), "owned");
}

#[test]
fn cow_slice() {
    let val: Cow<[u8]> = Cow::Owned(vec![10, 20, 30]);
    assert_eq!(&*roundtrip(&val), &[10, 20, 30]);
}

#[test]
fn rc_roundtrip() {
    let val = Rc::new(7u8);
    assert_eq!(*roundtrip(&val), 7u8);
}

#[test]
fn arc_roundtrip() {
    let val = Arc::new("shared".to_string());
    assert_eq!(*roundtrip(&val), "shared");
}

#[test]
fn result_ok() {
    let val: Result<u16, String> = Ok(42);
    assert_eq!(roundtrip(&val), Ok(42));
}

#[test]
fn result_err() {
    let val: Result<u16, String> = Err("oops".into());
    assert_eq!(roundtrip(&val), Err("oops".into()));
}

#[test]
fn result_discriminant_is_one_bit() {
    let ok: Result<(), ()> = Ok(());
    let err: Result<(), ()> = Err(());
    // both should encode to a single byte (1 bit + padding)
    assert_eq!(encode_to_vec(&ok).len(), 1);
    assert_eq!(encode_to_vec(&err).len(), 1);
}

#[test]
fn phantom_roundtrip() {
    let p: PhantomData<Vec<String>> = PhantomData;
    roundtrip(&p);
}

#[test]
fn phantom_zero_bytes() {
    assert!(encode_to_vec(&PhantomData::<u64>).is_empty());
}

#[test]
fn wrapping_roundtrip() {
    assert_eq!(roundtrip(&Wrapping(255u8)), Wrapping(255u8));
    assert_eq!(roundtrip(&Wrapping(i64::MIN)), Wrapping(i64::MIN));
}

#[test]
fn nonzero_roundtrip() {
    assert_eq!(
        roundtrip(&NonZeroU8::new(1).unwrap()),
        NonZeroU8::new(1).unwrap()
    );
    assert_eq!(
        roundtrip(&NonZeroU32::new(u32::MAX).unwrap()),
        NonZeroU32::new(u32::MAX).unwrap()
    );
    assert_eq!(
        roundtrip(&NonZeroI32::new(-1).unwrap()),
        NonZeroI32::new(-1).unwrap()
    );
}

#[test]
fn nonzero_decode_zero_fails() {
    let bytes = encode_to_vec(&0u8);
    assert!(decode_from_bytes::<NonZeroU8>(&bytes).is_err());

    let bytes = encode_to_vec(&0u32);
    assert!(decode_from_bytes::<NonZeroU32>(&bytes).is_err());

    let bytes = encode_to_vec(&0u64);
    assert!(decode_from_bytes::<NonZeroU64>(&bytes).is_err());
}

#[test]
fn char_roundtrip() {
    assert_eq!(roundtrip(&'A'), 'A');
    assert_eq!(roundtrip(&'🦀'), '🦀');
    assert_eq!(roundtrip(&'\0'), '\0');
    assert_eq!(roundtrip(&'\u{10FFFF}'), '\u{10FFFF}');
}

#[test]
fn char_decode_surrogate_fails() {
    let bytes = encode_to_vec(&0xD800u32);
    assert!(decode_from_bytes::<char>(&bytes).is_err());
}

#[test]
fn char_decode_out_of_range_fails() {
    let bytes = encode_to_vec(&0x110000u32);
    assert!(decode_from_bytes::<char>(&bytes).is_err());
}

#[test]
fn duration_roundtrip() {
    assert_eq!(
        roundtrip(&Duration::new(123, 456_789)),
        Duration::new(123, 456_789)
    );
    assert_eq!(roundtrip(&Duration::ZERO), Duration::ZERO);
    assert_eq!(
        roundtrip(&Duration::new(u64::MAX, 999_999_999)),
        Duration::new(u64::MAX, 999_999_999)
    );
}

#[test]
fn duration_decode_bad_nanos_fails() {
    // manually encode secs=0, nanos=1_000_000_000
    let mut w = BitWriter::new(Vec::new());
    0u64.encode(&mut w).unwrap();
    1_000_000_000u32.encode(&mut w).unwrap();
    w.flush().unwrap();
    assert!(decode_from_bytes::<Duration>(&w.into_inner()).is_err());
}

#[test]
fn ipv4_roundtrip() {
    assert_eq!(roundtrip(&Ipv4Addr::LOCALHOST), Ipv4Addr::LOCALHOST);
    assert_eq!(roundtrip(&Ipv4Addr::UNSPECIFIED), Ipv4Addr::UNSPECIFIED);
    assert_eq!(
        roundtrip(&Ipv4Addr::new(192, 168, 1, 1)),
        Ipv4Addr::new(192, 168, 1, 1)
    );
}

#[test]
fn ipv4_is_4_bytes() {
    assert_eq!(encode_to_vec(&Ipv4Addr::LOCALHOST).len(), 4);
}

#[test]
fn ipv6_roundtrip() {
    assert_eq!(roundtrip(&Ipv6Addr::LOCALHOST), Ipv6Addr::LOCALHOST);
    let addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
    assert_eq!(roundtrip(&addr), addr);
}

#[test]
fn ipv6_is_16_bytes() {
    assert_eq!(encode_to_vec(&Ipv6Addr::LOCALHOST).len(), 16);
}

#[test]
fn socket_addr_v4_roundtrip() {
    let addr = SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 8080);
    assert_eq!(roundtrip(&addr), addr);
}

#[test]
fn socket_addr_v6_roundtrip() {
    let addr = SocketAddrV6::new(Ipv6Addr::LOCALHOST, 443, 0, 0);
    assert_eq!(roundtrip(&addr), addr);
}

#[test]
fn socket_addr_enum_roundtrip() {
    let v4: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let v6: SocketAddr = "[::1]:443".parse().unwrap();
    assert_eq!(roundtrip(&v4), v4);
    assert_eq!(roundtrip(&v6), v6);
}

#[test]
fn socket_addr_v4_smaller_than_v6() {
    let v4: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let v6: SocketAddr = "[::]:0".parse().unwrap();
    assert!(encode_to_vec(&v4).len() < encode_to_vec(&v6).len());
}

#[test]
fn hashmap_roundtrip() {
    let mut m = HashMap::new();
    m.insert("a".to_string(), 1u32);
    m.insert("b".to_string(), 2);
    assert_eq!(roundtrip(&m), m);
}

#[test]
fn hashmap_empty() {
    let m: HashMap<u8, u8> = HashMap::new();
    assert_eq!(roundtrip(&m), m);
}

#[test]
fn btreemap_roundtrip() {
    let mut m = BTreeMap::new();
    m.insert(1u8, "one".to_string());
    m.insert(2, "two".to_string());
    m.insert(3, "three".to_string());
    assert_eq!(roundtrip(&m), m);
}

#[test]
fn btreemap_empty() {
    let m: BTreeMap<String, u32> = BTreeMap::new();
    assert_eq!(roundtrip(&m), m);
}

#[test]
fn hashset_roundtrip() {
    let s: HashSet<u32> = [1, 2, 3, 100].into_iter().collect();
    assert_eq!(roundtrip(&s), s);
}

#[test]
fn hashset_empty() {
    let s: HashSet<u8> = HashSet::new();
    assert_eq!(roundtrip(&s), s);
}

#[test]
fn btreeset_roundtrip() {
    let s: BTreeSet<i16> = [-10, 0, 10, 20].into_iter().collect();
    assert_eq!(roundtrip(&s), s);
}

#[test]
fn vecdeque_roundtrip() {
    let v: VecDeque<u8> = [10, 20, 30].into_iter().collect();
    assert_eq!(roundtrip(&v), v);
}

#[test]
fn vecdeque_empty() {
    let v: VecDeque<u32> = VecDeque::new();
    assert_eq!(roundtrip(&v), v);
}

#[test]
fn vecdeque_preserves_order() {
    let mut v = VecDeque::new();
    v.push_front(1u8);
    v.push_back(2);
    v.push_front(0);
    // logical order: [0, 1, 2]
    let decoded = roundtrip(&v);
    assert_eq!(decoded.iter().copied().collect::<Vec<_>>(), vec![0, 1, 2]);
}

#[test]
fn linked_list_roundtrip() {
    let l: LinkedList<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
    assert_eq!(roundtrip(&l), l);
}

#[test]
fn linked_list_empty() {
    let l: LinkedList<u8> = LinkedList::new();
    assert_eq!(roundtrip(&l), l);
}

#[test]
fn range_roundtrip() {
    assert_eq!(roundtrip(&(10u32..20u32)), 10..20);
    assert_eq!(roundtrip(&(-5i8..5i8)), -5..5);
}

#[test]
fn range_inclusive_roundtrip() {
    assert_eq!(roundtrip(&(1u16..=100u16)), 1..=100);
    assert_eq!(roundtrip(&(0i32..=0i32)), 0..=0);
}

#[test]
fn option_box_string() {
    let val: Option<Box<String>> = Some(Box::new("boxed".into()));
    assert_eq!(roundtrip(&val), val);
}

#[test]
fn vec_of_results() {
    let v: Vec<Result<u8, String>> = vec![Ok(1), Err("bad".into()), Ok(3)];
    assert_eq!(roundtrip(&v), v);
}

#[test]
fn map_of_ranges() {
    let mut m = BTreeMap::new();
    m.insert("a".to_string(), 0u32..10);
    m.insert("b".to_string(), 100..200);
    assert_eq!(roundtrip(&m), m);
}

#[test]
fn tuple_of_collections() {
    let val: (Vec<u8>, HashSet<u16>, BTreeMap<String, bool>) = (
        vec![1, 2, 3],
        [10, 20].into_iter().collect(),
        [("x".into(), true), ("y".into(), false)]
            .into_iter()
            .collect(),
    );
    assert_eq!(roundtrip(&val), val);
}

#[test]
fn deeply_nested() {
    #[allow(clippy::type_complexity)]
    let val: Vec<Option<Result<Box<(u8, String)>, NonZeroU16>>> = vec![
        Some(Ok(Box::new((42, "deep".into())))),
        Some(Err(NonZeroU16::new(999).unwrap())),
        None,
    ];
    assert_eq!(roundtrip(&val), val);
}
