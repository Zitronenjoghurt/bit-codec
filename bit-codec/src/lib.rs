use crate::var_len::VarLen;
pub use dec::{BitDecode, BitReader};
pub use enc::{BitEncode, BitWriter};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, LinkedList, VecDeque};
use std::hash::Hash;
use std::io::{self, Read, Write};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroU128, NonZeroU16, NonZeroU32,
    NonZeroU64, NonZeroU8, Wrapping,
};
use std::ops::{Range, RangeInclusive};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "derive")]
pub use bit_codec_derive::{BitDecode, BitEncode};

pub mod dec;
pub mod enc;
pub mod primitive;
pub mod seq;
#[cfg(test)]
mod tests;
pub mod var_len;

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

impl BitEncode for usize {
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        (*self as u64).encode(w)
    }
}

impl BitDecode for usize {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let val = u64::decode(r)?;
        usize::try_from(val).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "usize overflow on this platform",
            )
        })
    }
}

impl BitEncode for isize {
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        (*self as i64).encode(w)
    }
}

impl BitDecode for isize {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let val = i64::decode(r)?;
        isize::try_from(val).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "isize overflow on this platform",
            )
        })
    }
}

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

macro_rules! impl_tuple {
    ($($T:ident),+) => {
        impl<$($T: BitEncode),+> BitEncode for ($($T,)+) {
            fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
                #[allow(non_snake_case)]
                let ($($T,)+) = self;
                $($T.encode(w)?;)+
                Ok(())
            }
        }

        impl<$($T: BitDecode),+> BitDecode for ($($T,)+) {
            fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
                Ok(($($T::decode(r)?,)+))
            }
        }
    };
}

impl_tuple!(A);
impl_tuple!(A, B);
impl_tuple!(A, B, C);
impl_tuple!(A, B, C, D);
impl_tuple!(A, B, C, D, E);
impl_tuple!(A, B, C, D, E, F);
impl_tuple!(A, B, C, D, E, F, G);
impl_tuple!(A, B, C, D, E, F, G, H);
impl_tuple!(A, B, C, D, E, F, G, H, I);
impl_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);

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

impl BitEncode for str {
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        w.write(&self)
    }
}

impl BitEncode for () {
    #[inline]
    fn encode<W: Write>(&self, _w: &mut BitWriter<W>) -> io::Result<()> {
        Ok(())
    }
}

impl BitDecode for () {
    #[inline]
    fn decode<R: Read>(_r: &mut BitReader<R>) -> io::Result<Self> {
        Ok(())
    }
}

impl<T: BitEncode + ?Sized> BitEncode for Box<T> {
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        (**self).encode(w)
    }
}

impl<T: BitDecode> BitDecode for Box<T> {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        T::decode(r).map(Box::new)
    }
}

impl<T: BitDecode> BitDecode for Box<[T]> {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        Vec::<T>::decode(r).map(Vec::into_boxed_slice)
    }
}

impl BitDecode for Box<str> {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        String::decode(r).map(String::into_boxed_str)
    }
}

impl<'a, T> BitEncode for Cow<'a, [T]>
where
    T: Clone + BitEncode,
{
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        self.as_ref().encode(w)
    }
}

impl<'a, T> BitDecode for Cow<'a, [T]>
where
    T: Clone + BitDecode,
{
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        Vec::<T>::decode(r).map(Cow::Owned)
    }
}

impl<'a> BitEncode for Cow<'a, str> {
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        self.as_ref().encode(w)
    }
}

impl<'a> BitDecode for Cow<'a, str> {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        String::decode(r).map(Cow::Owned)
    }
}

impl<T: BitEncode + ?Sized> BitEncode for Rc<T> {
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        (**self).encode(w)
    }
}

impl<T: BitDecode> BitDecode for Rc<T> {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        T::decode(r).map(Rc::new)
    }
}

impl<T: BitEncode + ?Sized> BitEncode for Arc<T> {
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        (**self).encode(w)
    }
}

impl<T: BitDecode> BitDecode for Arc<T> {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        T::decode(r).map(Arc::new)
    }
}

impl<T: BitEncode, E: BitEncode> BitEncode for Result<T, E> {
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        match self {
            Ok(v) => {
                w.write(&true)?;
                v.encode(w)
            }
            Err(e) => {
                w.write(&false)?;
                e.encode(w)
            }
        }
    }
}

impl<T: BitDecode, E: BitDecode> BitDecode for Result<T, E> {
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        if r.read::<bool>()? {
            Ok(Ok(T::decode(r)?))
        } else {
            Ok(Err(E::decode(r)?))
        }
    }
}

impl<T> BitEncode for PhantomData<T> {
    #[inline]
    fn encode<W: Write>(&self, _w: &mut BitWriter<W>) -> io::Result<()> {
        Ok(())
    }
}

impl<T> BitDecode for PhantomData<T> {
    #[inline]
    fn decode<R: Read>(_r: &mut BitReader<R>) -> io::Result<Self> {
        Ok(PhantomData)
    }
}

impl<T: BitEncode> BitEncode for Wrapping<T> {
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        self.0.encode(w)
    }
}

impl<T: BitDecode> BitDecode for Wrapping<T> {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        T::decode(r).map(Wrapping)
    }
}

macro_rules! impl_nonzero {
    ($nz:ty, $inner:ty) => {
        impl BitEncode for $nz {
            #[inline]
            fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
                self.get().encode(w)
            }
        }

        impl BitDecode for $nz {
            #[inline]
            fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
                let val = <$inner>::decode(r)?;
                <$nz>::new(val).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        concat!(stringify!($nz), " cannot be zero"),
                    )
                })
            }
        }
    };
}

impl_nonzero!(NonZeroU8, u8);
impl_nonzero!(NonZeroU16, u16);
impl_nonzero!(NonZeroU32, u32);
impl_nonzero!(NonZeroU64, u64);
impl_nonzero!(NonZeroU128, u128);
impl_nonzero!(NonZeroI8, i8);
impl_nonzero!(NonZeroI16, i16);
impl_nonzero!(NonZeroI32, i32);
impl_nonzero!(NonZeroI64, i64);
impl_nonzero!(NonZeroI128, i128);

macro_rules! impl_seq_collection {
    ($ty:ident < T $(: $bound:ident)* >) => {
        impl<T: BitEncode> BitEncode for $ty<T> {
            fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
                VarLen(self.len() as u32).encode(w)?;
                for item in self {
                    item.encode(w)?;
                }
                Ok(())
            }
        }

        impl<T: BitDecode $(+ $bound)*> BitDecode for $ty<T> {
            fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
                let len = VarLen::decode(r)?.0 as usize;
                let mut col = $ty::new();
                for _ in 0..len {
                    col.push_back(T::decode(r)?);
                }
                Ok(col)
            }
        }
    };
}

impl_seq_collection!(VecDeque<T>);
impl_seq_collection!(LinkedList<T>);

impl<T: BitEncode> BitEncode for HashSet<T> {
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        VarLen(self.len() as u32).encode(w)?;
        for item in self {
            item.encode(w)?;
        }
        Ok(())
    }
}

impl<T: BitDecode + Eq + Hash> BitDecode for HashSet<T> {
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let len = VarLen::decode(r)?.0 as usize;
        let mut set = HashSet::with_capacity(len);
        for _ in 0..len {
            set.insert(T::decode(r)?);
        }
        Ok(set)
    }
}

impl<T: BitEncode> BitEncode for BTreeSet<T> {
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        VarLen(self.len() as u32).encode(w)?;
        for item in self {
            item.encode(w)?;
        }
        Ok(())
    }
}

impl<T: BitDecode + Ord> BitDecode for BTreeSet<T> {
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let len = VarLen::decode(r)?.0 as usize;
        let mut set = BTreeSet::new();
        for _ in 0..len {
            set.insert(T::decode(r)?);
        }
        Ok(set)
    }
}

impl<K: BitEncode, V: BitEncode> BitEncode for HashMap<K, V> {
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        VarLen(self.len() as u32).encode(w)?;
        for (k, v) in self {
            k.encode(w)?;
            v.encode(w)?;
        }
        Ok(())
    }
}

impl<K: BitDecode + Eq + Hash, V: BitDecode> BitDecode for HashMap<K, V> {
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let len = VarLen::decode(r)?.0 as usize;
        let mut map = HashMap::with_capacity(len);
        for _ in 0..len {
            map.insert(K::decode(r)?, V::decode(r)?);
        }
        Ok(map)
    }
}

impl<K: BitEncode, V: BitEncode> BitEncode for BTreeMap<K, V> {
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        VarLen(self.len() as u32).encode(w)?;
        for (k, v) in self {
            k.encode(w)?;
            v.encode(w)?;
        }
        Ok(())
    }
}

impl<K: BitDecode + Ord, V: BitDecode> BitDecode for BTreeMap<K, V> {
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let len = VarLen::decode(r)?.0 as usize;
        let mut map = BTreeMap::new();
        for _ in 0..len {
            map.insert(K::decode(r)?, V::decode(r)?);
        }
        Ok(map)
    }
}

impl BitEncode for char {
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        (*self as u32).encode(w)
    }
}

impl BitDecode for char {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let code = u32::decode(r)?;
        char::from_u32(code)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid char codepoint"))
    }
}

impl BitEncode for Duration {
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        self.as_secs().encode(w)?;
        self.subsec_nanos().encode(w)
    }
}

impl BitDecode for Duration {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let secs = u64::decode(r)?;
        let nanos = u32::decode(r)?;
        if nanos >= 1_000_000_000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Duration nanos >= 1 billion",
            ));
        }
        Ok(Duration::new(secs, nanos))
    }
}

impl BitEncode for Ipv4Addr {
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        w.write_full_bytes(&self.octets())
    }
}

impl BitDecode for Ipv4Addr {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let mut buf = [0u8; 4];
        r.read_full_bytes(&mut buf)?;
        Ok(Ipv4Addr::from(buf))
    }
}

impl BitEncode for Ipv6Addr {
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        w.write_full_bytes(&self.octets())
    }
}

impl BitDecode for Ipv6Addr {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let mut buf = [0u8; 16];
        r.read_full_bytes(&mut buf)?;
        Ok(Ipv6Addr::from(buf))
    }
}

impl BitEncode for SocketAddrV4 {
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        self.ip().encode(w)?;
        self.port().encode(w)
    }
}

impl BitDecode for SocketAddrV4 {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let ip = Ipv4Addr::decode(r)?;
        let port = u16::decode(r)?;
        Ok(SocketAddrV4::new(ip, port))
    }
}

impl BitEncode for SocketAddrV6 {
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        self.ip().encode(w)?;
        self.port().encode(w)
    }
}

impl BitDecode for SocketAddrV6 {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let ip = Ipv6Addr::decode(r)?;
        let port = u16::decode(r)?;
        Ok(SocketAddrV6::new(ip, port, 0, 0))
    }
}

impl BitEncode for SocketAddr {
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        match self {
            SocketAddr::V4(a) => {
                w.write_bits(0u8, 1)?;
                a.encode(w)
            }
            SocketAddr::V6(a) => {
                w.write_bits(1u8, 1)?;
                a.encode(w)
            }
        }
    }
}

impl BitDecode for SocketAddr {
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        match r.read_bits::<u8>(1)? {
            0 => SocketAddrV4::decode(r).map(SocketAddr::V4),
            _ => SocketAddrV6::decode(r).map(SocketAddr::V6),
        }
    }
}

impl<T: BitEncode> BitEncode for Range<T> {
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        self.start.encode(w)?;
        self.end.encode(w)
    }
}

impl<T: BitDecode> BitDecode for Range<T> {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let start = T::decode(r)?;
        let end = T::decode(r)?;
        Ok(start..end)
    }
}

impl<T: BitEncode> BitEncode for RangeInclusive<T> {
    #[inline]
    fn encode<W: Write>(&self, w: &mut BitWriter<W>) -> io::Result<()> {
        self.start().encode(w)?;
        self.end().encode(w)
    }
}

impl<T: BitDecode> BitDecode for RangeInclusive<T> {
    #[inline]
    fn decode<R: Read>(r: &mut BitReader<R>) -> io::Result<Self> {
        let start = T::decode(r)?;
        let end = T::decode(r)?;
        Ok(start..=end)
    }
}
