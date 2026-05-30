use bit_codec::{BitDecode, BitEncode, BitReader, BitWriter};

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
    Print(String),
}

#[derive(Debug, PartialEq, BitEncode, BitDecode)]
struct Frame {
    #[bits(4)]
    version: u8,
    active: bool,
    commands: Vec<Command>,
}

fn main() {
    let frame = Frame {
        version: 3,
        active: true,
        commands: vec![
            Command::SetColor(Color { r: 31, g: 63, b: 0 }),
            Command::Move { dx: -10, dy: 20 },
            Command::Print("hello bit-codec!".into()),
            Command::Nop,
        ],
    };

    let mut buf = Vec::new();
    let mut w = BitWriter::new(&mut buf);
    w.write(&frame).unwrap();
    w.flush().unwrap();
    drop(w);

    println!("Encoded {} bytes", buf.len());

    let mut r = BitReader::new(buf.as_slice());
    let decoded = r.read::<Frame>().unwrap();

    println!("{decoded:#?}");
    assert_eq!(frame, decoded);

    let mut color_buf = Vec::new();
    let mut w = BitWriter::new(&mut color_buf);
    w.write(&Color {
        r: 31,
        g: 63,
        b: 31,
    })
    .unwrap();
    w.flush().unwrap();
    drop(w);
    println!(
        "Color: 3 fields, only {} bytes on the wire",
        color_buf.len()
    );
}
