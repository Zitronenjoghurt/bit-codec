use crate::primitive::BitInt;
use std::io::{self, Read};

pub trait BitDecode: Sized {
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self>;
}

pub struct BitReader<R: Read> {
    inner: R,
    buf: u8,
    bits_left: u8,
}

impl<R: Read> BitReader<R> {
    #[inline]
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            buf: 0,
            bits_left: 0,
        }
    }

    #[inline]
    pub fn read<T: BitDecode>(&mut self) -> io::Result<T> {
        T::decode(self)
    }

    #[inline]
    pub fn read_bits<T: BitInt>(&mut self, count: u32) -> io::Result<T> {
        if count == 0 || count > T::BITS {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "bad bit count"));
        }
        let mut acc = T::ZERO;
        let mut remaining = count;
        while remaining > 0 {
            if self.bits_left == 0 {
                let mut b = [0u8];
                self.inner.read_exact(&mut b)?;
                self.buf = b[0];
                self.bits_left = 8;
            }
            let chunk = remaining.min(self.bits_left as u32);
            let lo_shift = self.bits_left as u32 - chunk;
            let mask: u8 = if chunk == 8 { 0xFF } else { (1u8 << chunk) - 1 };
            let bits = (self.buf >> lo_shift) & mask;
            remaining -= chunk;
            acc = acc.deposit_byte(bits, remaining);
            self.bits_left -= chunk as u8;
        }
        Ok(acc)
    }

    pub fn read_full_bytes(&mut self, out: &mut [u8]) -> io::Result<()> {
        if self.is_aligned() {
            self.inner.read_exact(out)
        } else {
            for byte in out.iter_mut() {
                *byte = self.read_bits(8)?;
            }
            Ok(())
        }
    }

    pub fn is_aligned(&self) -> bool {
        self.bits_left == 0
    }

    pub fn align(&mut self) {
        self.bits_left = 0;
    }
}

impl<R: Read> Read for BitReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        assert!(self.is_aligned(), "raw Read on unaligned BitReader");
        self.inner.read(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BitWriter;

    #[test]
    fn test_read_bits_zero_count_errors() {
        let mut r = BitReader::new(&[0u8][..]);
        assert!(r.read_bits::<u8>(0).is_err());
    }

    #[test]
    fn test_read_bits_over_type_width_errors() {
        let mut r = BitReader::new(&[0u8, 0u8][..]);
        assert!(r.read_bits::<u8>(9).is_err());
    }

    #[test]
    fn test_cross_byte_boundary() {
        let mut w = BitWriter::new(Vec::new());
        w.write_bits(0b10101u8, 5).unwrap();
        w.write_bits(0b11001100u8, 8).unwrap();
        w.flush().unwrap();
        let bytes = w.into_inner();

        let mut r = BitReader::new(&bytes[..]);
        assert_eq!(r.read_bits::<u8>(5).unwrap(), 0b10101);
        assert_eq!(r.read_bits::<u8>(8).unwrap(), 0b11001100);
    }

    #[test]
    fn test_eof_mid_read() {
        let mut r = BitReader::new(&[0xAB][..]);
        r.read_bits::<u8>(8).unwrap();
        assert!(r.read_bits::<u8>(1).is_err());
    }

    #[test]
    fn test_read_full_bytes_unaligned() {
        let mut w = BitWriter::new(Vec::new());
        w.write_bits(1u8, 1).unwrap();
        w.write_full_bytes(&[0xAB, 0xCD]).unwrap();
        w.flush().unwrap();
        let bytes = w.into_inner();

        let mut r = BitReader::new(&bytes[..]);
        assert_eq!(r.read_bits::<u8>(1).unwrap(), 1);
        let mut out = [0u8; 2];
        r.read_full_bytes(&mut out).unwrap();
        assert_eq!(out, [0xAB, 0xCD]);
    }

    #[test]
    fn test_align_discards_remaining_bits() {
        let mut r = BitReader::new(&[0xFF, 0x80][..]);
        r.read_bits::<u8>(3).unwrap();
        r.align();
        assert_eq!(r.read_bits::<u8>(1).unwrap(), 1);
    }
}
