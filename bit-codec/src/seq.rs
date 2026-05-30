use crate::{BitDecode, BitEncode, BitReader, BitWriter};
use std::io::{Read, Write};
use std::marker::PhantomData;

/// A Vec that encodes its length as `H`.
pub struct Seq<H, T> {
    pub inner: Vec<T>,
    _header: PhantomData<H>,
}

impl<H, T> Seq<H, T> {
    pub fn new(inner: Vec<T>) -> Self {
        Self {
            inner,
            _header: PhantomData,
        }
    }
}

impl<H, T: BitEncode> BitEncode for Seq<H, T>
where
    H: BitEncode + TryFrom<usize>,
{
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> std::io::Result<()> {
        let len = H::try_from(self.inner.len()).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "failed to encode sequence length",
            )
        })?;
        len.encode(w)?;
        for item in &self.inner {
            item.encode(w)?;
        }
        Ok(())
    }
}

impl<H, T: BitDecode> BitDecode for Seq<H, T>
where
    H: BitDecode + TryInto<usize>,
{
    fn decode<R: Read>(r: &mut BitReader<R>) -> std::io::Result<Self> {
        let raw = H::decode(r)?;
        let len: usize = raw.try_into().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "failed to decode sequence length",
            )
        })?;
        let inner = (0..len)
            .map(|_| T::decode(r))
            .collect::<std::io::Result<_>>()?;
        Ok(Self {
            inner,
            _header: PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{decode_from_bytes, encode_to_vec, roundtrip};

    #[test]
    fn test_empty() {
        let s: Seq<u16, u32> = Seq::new(vec![]);
        let bytes = encode_to_vec(&s);
        let decoded: Seq<u16, u32> = decode_from_bytes(&bytes).unwrap();
        assert!(decoded.inner.is_empty());
    }

    #[test]
    fn test_roundtrip() {
        let s: Seq<u8, u16> = Seq::new(vec![100, 200, 300]);
        let decoded: Seq<u8, u16> = roundtrip(&s);
        assert_eq!(decoded.inner, vec![100, 200, 300]);
    }

    #[test]
    fn test_length_overflow_encode() {
        let s: Seq<u8, u8> = Seq::new(vec![0; 256]);
        let mut w = BitWriter::new(Vec::new());
        assert!(
            s.encode(&mut w).is_err(),
            "should reject length that overflows H"
        );
    }
}
