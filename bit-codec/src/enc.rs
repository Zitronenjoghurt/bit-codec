use crate::primitive::BitInt;
use std::io::{self, Write};

pub trait BitEncode {
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()>;
}

pub struct BitWriter<W: Write> {
    inner: Option<W>,
    current_byte: u8,
    bit_count: u8,
}

impl<W: Write> BitWriter<W> {
    #[inline]
    pub fn new(inner: W) -> Self {
        Self {
            inner: Some(inner),
            current_byte: 0,
            bit_count: 0,
        }
    }

    #[inline]
    pub fn write<T: BitEncode + ?Sized>(&mut self, v: &T) -> io::Result<()> {
        v.encode(self)
    }

    #[inline]
    pub fn write_bits<T: BitInt>(&mut self, value: T, count: u32) -> io::Result<()> {
        if count == 0 || count > T::BITS {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "bad bit count"));
        }
        let mut remaining = count;
        while remaining > 0 {
            let space = 8 - self.bit_count as u32;
            let chunk = remaining.min(space);
            let shift = remaining - chunk;
            let mask: u8 = if chunk == 8 { 0xFF } else { (1u8 << chunk) - 1 };
            let bits = value.extract_byte(shift) & mask;
            self.current_byte |= bits << (space - chunk);
            self.bit_count += chunk as u8;
            if self.bit_count == 8 {
                self.inner
                    .as_mut()
                    .unwrap()
                    .write_all(&[self.current_byte])?;
                self.current_byte = 0;
                self.bit_count = 0;
            }
            remaining -= chunk;
        }
        Ok(())
    }

    pub fn write_full_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        if self.is_aligned() {
            self.inner.as_mut().unwrap().write_all(bytes)?;
        } else {
            for &b in bytes {
                self.write_bits(b, 8)?;
            }
        }
        Ok(())
    }

    #[inline]
    pub fn is_aligned(&self) -> bool {
        self.bit_count == 0
    }

    /// Pad remaining bits with zeros to reach a byte boundary.
    pub fn align(&mut self) -> io::Result<()> {
        if self.bit_count > 0 {
            self.inner
                .as_mut()
                .unwrap()
                .write_all(&[self.current_byte])?;
            self.current_byte = 0;
            self.bit_count = 0;
        }
        Ok(())
    }

    /// Pad to byte boundary and flush the underlying writer.
    /// Always call this when done writing.
    pub fn flush(&mut self) -> io::Result<()> {
        self.align()?;
        self.inner.as_mut().unwrap().flush()
    }

    /// Consume the writer. Does NOT auto-flush — call `.flush()` first.
    pub fn into_inner(mut self) -> W {
        self.inner.take().unwrap()
    }
}

impl<W: Write> Write for BitWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        assert!(self.is_aligned(), "raw Write on unaligned BitWriter");
        self.inner.as_mut().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush()
    }
}

impl<W: Write> Drop for BitWriter<W> {
    fn drop(&mut self) {
        if self.bit_count > 0 && !std::thread::panicking() {
            debug_assert!(false, "BitWriter dropped with unflushed bits");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flush_is_not_infinite_recursion() {
        let mut w = BitWriter::new(Vec::new());
        w.write_bits(1u8, 4).unwrap();
        use std::io::Write;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Write::flush(&mut w).unwrap();
        }));
        assert!(result.is_ok(), "Write::flush likely hit infinite recursion");
    }

    #[test]
    fn test_write_bits_zero_count_errors() {
        let mut w = BitWriter::new(Vec::new());
        assert!(w.write_bits(0u8, 0).is_err());
    }

    #[test]
    fn test_write_bits_over_type_width_errors() {
        let mut w = BitWriter::new(Vec::new());
        assert!(w.write_bits(0u8, 9).is_err());
    }

    #[test]
    fn test_align_pads_with_zeros() {
        let mut w = BitWriter::new(Vec::new());
        w.write_bits(0b111u8, 3).unwrap();
        w.align().unwrap();
        let bytes = w.into_inner();
        assert_eq!(bytes, vec![0xE0]);
    }

    #[test]
    fn test_raw_write_unaligned_panics() {
        let mut w = BitWriter::new(Vec::new());
        w.write_bits(1u8, 1).unwrap();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = std::io::Write::write(&mut w, &[0xAB]);
        }));

        assert!(result.is_err(), "raw Write on unaligned should panic");
        w.align().unwrap();
    }
}
