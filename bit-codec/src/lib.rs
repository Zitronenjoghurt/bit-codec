use crate::var_len::VarLen;
pub use dec::{BitDecode, BitReader};
pub use enc::{BitEncode, BitWriter};
use std::io::{self, Read, Write};
use std::mem::MaybeUninit;

#[cfg(feature = "derive")]
pub use bit_codec_derive::{BitDecode, BitEncode};

pub mod dec;
pub mod enc;
mod primitive;
mod seq;
#[cfg(test)]
mod tests;
mod var_len;

macro_rules! impl_bit_codec {
    ($($ty:ty),*) => {
        $(
            impl BitEncode for $ty {
                #[inline]
                fn encode<W: std::io::Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
                    w.write_full_bytes(&self.to_le_bytes())
                }
            }
            impl BitDecode for $ty {
                #[inline]
                fn decode<R: std::io::Read>(r: &mut BitReader<R>) -> io::Result<Self> {
                    let mut buf = [0u8; std::mem::size_of::<$ty>()];
                    r.read_full_bytes(&mut buf)?;
                    Ok(<$ty>::from_le_bytes(buf))
                }
            }
        )*
    };
}
impl_bit_codec!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

impl<T: BitEncode> BitEncode for [T] {
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        VarLen(self.len() as u32).encode(w)?;
        for item in self {
            item.encode(w)?;
        }
        Ok(())
    }
}

impl<T: BitEncode> BitEncode for Vec<T> {
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        self.as_slice().encode(w)
    }
}

impl<T: BitDecode> BitDecode for Vec<T> {
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let len = VarLen::decode(r)?.0 as usize;
        (0..len).map(|_| T::decode(r)).collect()
    }
}

impl<T: BitEncode, const N: usize> BitEncode for [T; N] {
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        for item in self {
            item.encode(w)?;
        }
        Ok(())
    }
}

impl<T: BitDecode, const N: usize> BitDecode for [T; N] {
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        // SAFETY: we immediately overwrite the garbage values and dont access them as T before it completes successfully
        let mut arr: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };

        for (i, slot) in arr.iter_mut().enumerate() {
            match T::decode(r) {
                Ok(val) => {
                    slot.write(val);
                }
                Err(e) => {
                    // SAFETY: we only call drop on slots we wrote a T into to prevent leaking memory
                    for slot in &mut arr[..i] {
                        unsafe {
                            slot.assume_init_drop();
                        }
                    }
                    return Err(e);
                }
            }
        }

        // SAFETY: the loop finished without error, so every slot got a proper T
        Ok(unsafe { arr.map(|slot| slot.assume_init()) })
    }
}

impl<T: BitEncode> BitEncode for Option<T> {
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        w.write(&self.is_some())?;
        if let Some(v) = self {
            v.encode(w)?;
        }
        Ok(())
    }
}

impl<T: BitDecode> BitDecode for Option<T> {
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        if r.read::<bool>()? {
            Ok(Some(T::decode(r)?))
        } else {
            Ok(None)
        }
    }
}

impl BitEncode for bool {
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        w.write_bits(*self as u8, 1)
    }
}

impl BitDecode for bool {
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        r.read_bits::<u8>(1).map(|b| b != 0)
    }
}

impl BitEncode for String {
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        VarLen(self.len() as u32).encode(w)?;
        w.write_full_bytes(self.as_bytes())
    }
}

impl BitDecode for String {
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let len = VarLen::decode(r)?.0 as usize;
        let mut buf = vec![0u8; len];
        r.read_full_bytes(&mut buf)?;
        String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

impl BitEncode for &str {
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        VarLen(self.len() as u32).encode(w)?;
        w.write_full_bytes(self.as_bytes())
    }
}
