use crate::{BitDecode, BitEncode, BitReader, BitWriter};

pub struct VarLen(pub u32);

impl BitEncode for VarLen {
    fn encode<W: std::io::Write>(&self, w: &mut BitWriter<W>) -> std::io::Result<()> {
        let mut val = self.0;
        loop {
            if val < 128 {
                w.write_bits(0u8, 1)?;
                w.write_bits(val as u8, 7)?;
                return Ok(());
            } else {
                w.write_bits(1u8, 1)?;
                w.write_bits(val as u8, 7)?;
                val >>= 7;
            }
        }
    }
}

impl BitDecode for VarLen {
    fn decode<R: std::io::Read>(r: &mut BitReader<R>) -> std::io::Result<Self> {
        let mut val = 0u32;
        let mut shift = 0;
        loop {
            let cont = r.read_bits::<u8>(1)?;
            let chunk = r.read_bits::<u8>(7)? as u32;
            if shift == 28 && chunk > 0x0F {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "VarLen overflow: upper bits truncated",
                ));
            }

            val |= chunk << shift;

            if cont == 0 {
                return Ok(VarLen(val));
            }

            shift += 7;
            if shift >= 32 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "VarLen overflow: too many bytes",
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::var_len::VarLen;
    use crate::BitWriter;

    #[test]
    fn test_zero() {
        let rt = crate::tests::roundtrip(&VarLen(0));
        assert_eq!(rt.0, 0);
    }

    #[test]
    fn test_one_byte_max() {
        let rt = crate::tests::roundtrip(&VarLen(127));
        assert_eq!(rt.0, 127);
    }

    #[test]
    fn test_two_byte_min() {
        let rt = crate::tests::roundtrip(&VarLen(128));
        assert_eq!(rt.0, 128);
    }

    #[test]
    fn test_u32_max() {
        let rt = crate::tests::roundtrip(&VarLen(u32::MAX));
        assert_eq!(rt.0, u32::MAX);
    }

    #[test]
    fn test_each_boundary() {
        for &val in &[
            0,
            127,
            128,
            16383,
            16384,
            2097151,
            2097152,
            268435455,
            268435456,
            u32::MAX,
        ] {
            let rt = crate::tests::roundtrip(&VarLen(val));
            assert_eq!(rt.0, val, "roundtrip failed for {val}");
        }
    }

    #[test]
    fn test_decode_overflow_crafted() {
        let mut w = BitWriter::new(Vec::new());
        for _ in 0..6 {
            w.write_bits(1u8, 1).unwrap();
            w.write_bits(1u8, 7).unwrap();
        }
        w.flush().unwrap();
        let bytes = w.into_inner();
        let result = crate::tests::decode_from_bytes::<VarLen>(&bytes);
        assert!(
            result.is_err(),
            "should reject VarLen with too many continuation bytes"
        );
    }

    #[test]
    fn test_decode_upper_bits_overflow() {
        let mut w = BitWriter::new(Vec::new());
        for _ in 0..4 {
            w.write_bits(1u8, 1).unwrap();
            w.write_bits(0u8, 7).unwrap();
        }
        w.write_bits(0u8, 1).unwrap();
        w.write_bits(0x10u8, 7).unwrap();
        w.flush().unwrap();
        let bytes = w.into_inner();

        let result = crate::tests::decode_from_bytes::<VarLen>(&bytes);
        if let Ok(v) = result {
            assert_eq!(
                v.0, 0,
                "decoder silently truncated overflow bits — this is a bug"
            );
            panic!(
                "VarLen decoder should reject values that overflow u32, but it silently truncated"
            );
        }
    }
}
