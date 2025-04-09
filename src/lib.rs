pub mod ints;
pub use bitstuff_macros::*;

pub trait FromLowBits<T> {
    fn from_low_bits(bits: T) -> Self;
}

impl FromLowBits<u8> for bool {
    fn from_low_bits(bits: u8) -> Self {
        bits & 1 != 0
    }
}

impl FromLowBits<u8> for u8 {
    fn from_low_bits(bits: u8) -> Self {
        bits
    }
}
impl FromLowBits<u16> for u16 {
    fn from_low_bits(bits: u16) -> Self {
        bits
    }
}
impl FromLowBits<u32> for u32 {
    fn from_low_bits(bits: u32) -> Self {
        bits
    }
}
impl FromLowBits<u64> for u64 {
    fn from_low_bits(bits: u64) -> Self {
        bits
    }
}
impl FromLowBits<u128> for u128 {
    fn from_low_bits(bits: u128) -> Self {
        bits
    }
}
impl<T> FromLowBits<u128> for T
where
    T: FromLowBits<u64>,
{
    fn from_low_bits(bits: u128) -> Self {
        T::from_low_bits(bits as u64)
    }
}
impl<T> FromLowBits<u64> for T
where
    T: FromLowBits<u32>,
{
    fn from_low_bits(bits: u64) -> Self {
        T::from_low_bits(bits as u32)
    }
}
impl<T> FromLowBits<u32> for T
where
    T: FromLowBits<u16>,
{
    fn from_low_bits(bits: u32) -> Self {
        T::from_low_bits(bits as u16)
    }
}
impl<T> FromLowBits<u16> for T
where
    T: FromLowBits<u8>,
{
    fn from_low_bits(bits: u16) -> Self {
        T::from_low_bits(bits as u8)
    }
}

pub trait Bits {
    const N_BITS: u32;
}

impl Bits for bool {
    const N_BITS: u32 = 1;
}
impl Bits for u8 {
    const N_BITS: u32 = 8;
}

impl Bits for u16 {
    const N_BITS: u32 = 16;
}

impl Bits for u32 {
    const N_BITS: u32 = 32;
}

impl Bits for u64 {
    const N_BITS: u32 = 64;
}

impl Bits for u128 {
    const N_BITS: u32 = 128;
}

pub trait ToBits: Bits {
    type To;
    fn to_bits(self) -> Self::To;
}

impl ToBits for u8 {
    type To = u8;
    fn to_bits(self) -> u8 {
        self
    }
}

impl ToBits for u16 {
    type To = u16;
    fn to_bits(self) -> u16 {
        self
    }
}
impl ToBits for u32 {
    type To = u32;
    fn to_bits(self) -> u32 {
        self
    }
}
impl ToBits for u64 {
    type To = u64;
    fn to_bits(self) -> u64 {
        self
    }
}
impl ToBits for u128 {
    type To = u128;
    fn to_bits(self) -> u128 {
        self
    }
}
impl ToBits for bool {
    type To = u8;
    fn to_bits(self) -> u8 {
        if self {
            1u8
        } else {
            0u8
        }
    }
}

pub trait FromBits: Bits {
    type From;
    fn from_bits(bits: Self::From) -> Self;
}

impl FromBits for u8 {
    type From = u8;
    fn from_bits(bits: u8) -> Self {
        bits
    }
}
impl FromBits for u16 {
    type From = u16;
    fn from_bits(bits: u16) -> Self {
        bits
    }
}
impl FromBits for u32 {
    type From = u32;
    fn from_bits(bits: u32) -> Self {
        bits
    }
}
impl FromBits for u64 {
    type From = u64;
    fn from_bits(bits: u64) -> Self {
        bits
    }
}
impl FromBits for u128 {
    type From = u128;
    fn from_bits(bits: u128) -> Self {
        bits
    }
}
impl FromBits for bool {
    type From = u8;
    fn from_bits(bits: u8) -> Self {
        bits & 1 != 0
    }
}

pub trait TryFromBits: Bits + Sized {
    type From;
    fn try_from_bits(bits: Self::From) -> Result<Self, Self::From>;
}

impl<S: TryFromBits> Bits for Result<S, S::From> {
    const N_BITS: u32 = S::N_BITS;
}
