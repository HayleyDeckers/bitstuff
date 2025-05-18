#![no_std]
pub mod ints;
use core::num::{NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8};

pub use bitstuff_macros::*;
use ints::u1;

pub trait BitRepr {
    type BitRepr;
}

pub trait FromBits: BitRepr {
    fn from_bits(bits: Self::BitRepr) -> Self;
}

impl<T> FromBits for T
where
    T: BitRepr<BitRepr = T>,
{
    fn from_bits(bits: T::BitRepr) -> Self {
        bits.into()
    }
}

impl BitRepr for bool {
    type BitRepr = u1;
}
impl BitRepr for u8 {
    type BitRepr = u8;
}
impl BitRepr for u16 {
    type BitRepr = u16;
}
impl BitRepr for u32 {
    type BitRepr = u32;
}
impl BitRepr for u64 {
    type BitRepr = u64;
}
impl BitRepr for u128 {
    type BitRepr = u128;
}
impl BitRepr for NonZeroU8 {
    type BitRepr = u8;
}
impl BitRepr for NonZeroU16 {
    type BitRepr = u16;
}
impl BitRepr for NonZeroU32 {
    type BitRepr = u32;
}
impl BitRepr for NonZeroU64 {
    type BitRepr = u64;
}
impl BitRepr for NonZeroU128 {
    type BitRepr = u128;
}

impl FromBits for bool {
    fn from_bits(bits: u1) -> Self {
        *bits != 0
    }
}

pub trait ToBits: BitRepr {
    fn to_bits(self) -> Self::BitRepr;
}

impl<T> ToBits for T
where
    T: BitRepr<BitRepr = T>,
{
    fn to_bits(self) -> Self::BitRepr {
        self
    }
}

impl ToBits for bool {
    fn to_bits(self) -> u1 {
        u1::trimmed_new(self as u8)
    }
}

pub trait TryFromBits: BitRepr + Sized {
    fn try_from_bits(bits: Self::BitRepr) -> Result<Self, Self::BitRepr>;
}

impl TryFromBits for NonZeroU8 {
    fn try_from_bits(bits: u8) -> Result<Self, u8> {
        NonZeroU8::new(bits).ok_or(bits)
    }
}

impl TryFromBits for NonZeroU16 {
    fn try_from_bits(bits: u16) -> Result<Self, u16> {
        NonZeroU16::new(bits).ok_or(bits)
    }
}

impl TryFromBits for NonZeroU32 {
    fn try_from_bits(bits: u32) -> Result<Self, u32> {
        NonZeroU32::new(bits).ok_or(bits)
    }
}

impl TryFromBits for NonZeroU64 {
    fn try_from_bits(bits: u64) -> Result<Self, u64> {
        NonZeroU64::new(bits).ok_or(bits)
    }
}

impl TryFromBits for NonZeroU128 {
    fn try_from_bits(bits: u128) -> Result<Self, u128> {
        NonZeroU128::new(bits).ok_or(bits)
    }
}

impl ToBits for NonZeroU8 {
    fn to_bits(self) -> u8 {
        self.get()
    }
}
impl ToBits for NonZeroU16 {
    fn to_bits(self) -> u16 {
        self.get()
    }
}
impl ToBits for NonZeroU32 {
    fn to_bits(self) -> u32 {
        self.get()
    }
}
impl ToBits for NonZeroU64 {
    fn to_bits(self) -> u64 {
        self.get()
    }
}
impl ToBits for NonZeroU128 {
    fn to_bits(self) -> u128 {
        self.get()
    }
}

impl BitRepr for Option<NonZeroU8> {
    type BitRepr = u8;
}
impl FromBits for Option<NonZeroU8> {
    fn from_bits(bits: u8) -> Self {
        NonZeroU8::new(bits)
    }
}
impl ToBits for Option<NonZeroU8> {
    fn to_bits(self) -> u8 {
        self.map_or(0, |v| v.get())
    }
}

impl BitRepr for Option<NonZeroU16> {
    type BitRepr = u16;
}
impl FromBits for Option<NonZeroU16> {
    fn from_bits(bits: u16) -> Self {
        NonZeroU16::new(bits)
    }
}
impl ToBits for Option<NonZeroU16> {
    fn to_bits(self) -> u16 {
        self.map_or(0, |v| v.get())
    }
}

impl BitRepr for Option<NonZeroU32> {
    type BitRepr = u32;
}
impl FromBits for Option<NonZeroU32> {
    fn from_bits(bits: u32) -> Self {
        NonZeroU32::new(bits)
    }
}
impl ToBits for Option<NonZeroU32> {
    fn to_bits(self) -> u32 {
        self.map_or(0, |v| v.get())
    }
}

impl BitRepr for Option<NonZeroU64> {
    type BitRepr = u64;
}
impl FromBits for Option<NonZeroU64> {
    fn from_bits(bits: u64) -> Self {
        NonZeroU64::new(bits)
    }
}
impl ToBits for Option<NonZeroU64> {
    fn to_bits(self) -> u64 {
        self.map_or(0, |v| v.get())
    }
}

impl BitRepr for Option<NonZeroU128> {
    type BitRepr = u128;
}
impl FromBits for Option<NonZeroU128> {
    fn from_bits(bits: u128) -> Self {
        NonZeroU128::new(bits)
    }
}
impl ToBits for Option<NonZeroU128> {
    fn to_bits(self) -> u128 {
        self.map_or(0, |v| v.get())
    }
}
