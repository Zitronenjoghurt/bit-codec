use crate::{BitDecode, BitEncode, BitReader, BitWriter};

#[derive(Debug, PartialEq, BitEncode, BitDecode)]
struct Pair(u16, u16);

#[derive(Debug, PartialEq, BitEncode, BitDecode)]
struct Mixed {
    #[bits(4)]
    nibble: u8,
    flag: bool,
    label: String,
}

#[derive(Debug, PartialEq, BitEncode, BitDecode)]
#[bits(disc = 2)]
enum Small {
    A,
    B(u8),
    C { x: u16, y: u16 },
}

#[derive(Debug, PartialEq, BitEncode, BitDecode)]
struct Color {
    #[bits(5)]
    r: u8,
    #[bits(6)]
    g: u8,
    #[bits(5)]
    b: u8,
}

#[derive(Debug, PartialEq, BitEncode, BitDecode)]
#[bits(disc = 4)]
enum Command {
    Nop,
    SetColor(Color),
    Move { dx: i16, dy: i16 },
    Batch { label: String, steps: Vec<Small> },
}

#[derive(Debug, PartialEq, BitEncode, BitDecode)]
struct Frame {
    #[bits(4)]
    version: u8,
    active: bool,
    origin: Pair,
    bg: Color,
    commands: Vec<Command>,
}

fn round_trip<T: BitEncode + BitDecode + std::fmt::Debug + PartialEq>(val: &T) {
    let mut buf = Vec::new();
    let mut w = BitWriter::new(&mut buf);
    w.write(val).unwrap();
    w.flush().unwrap();
    drop(w);

    let mut r = BitReader::new(buf.as_slice());
    let decoded = r.read::<T>().unwrap();
    assert_eq!(*val, decoded);
}

#[test]
fn struct_named() {
    round_trip(&Mixed {
        nibble: 0xA,
        flag: true,
        label: "hi".into(),
    });
}

#[test]
fn struct_tuple() {
    round_trip(&Pair(100, 200));
}

#[test]
fn enum_unit() {
    round_trip(&Small::A);
}

#[test]
fn enum_tuple() {
    round_trip(&Small::B(42));
}

#[test]
fn enum_struct() {
    round_trip(&Small::C { x: 1, y: 2 });
}

#[test]
fn packed_color() {
    round_trip(&Color { r: 31, g: 63, b: 0 });
}

#[test]
fn nested_enum_in_struct() {
    round_trip(&Frame {
        version: 7,
        active: true,
        origin: Pair(320, 240),
        bg: Color {
            r: 10,
            g: 20,
            b: 15,
        },
        commands: vec![
            Command::Nop,
            Command::SetColor(Color { r: 31, g: 0, b: 31 }),
            Command::Move { dx: -100, dy: 50 },
        ],
    });
}

#[test]
fn deeply_nested_vec() {
    round_trip(&Frame {
        version: 1,
        active: false,
        origin: Pair(0, 0),
        bg: Color { r: 0, g: 0, b: 0 },
        commands: vec![
            Command::Batch {
                label: "init".into(),
                steps: vec![Small::A, Small::B(255), Small::C { x: 1000, y: 2000 }],
            },
            Command::Batch {
                label: "empty".into(),
                steps: vec![],
            },
        ],
    });
}

#[test]
fn empty_frame() {
    round_trip(&Frame {
        version: 0,
        active: false,
        origin: Pair(0, 0),
        bg: Color { r: 0, g: 0, b: 0 },
        commands: vec![],
    });
}

#[test]
fn bad_discriminant() {
    let mut buf = Vec::new();
    let mut w = BitWriter::new(&mut buf);
    w.write_bits(15u32, 4).unwrap(); // discriminant 15, no such Command variant
    w.flush().unwrap();
    drop(w);

    let mut r = BitReader::new(buf.as_slice());
    let err = r.read::<Command>().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}
