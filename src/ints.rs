#![allow(non_camel_case_types)]

pub mod implementation {
    use core::ops::Deref;
    use core::panic;

    use crate::BitRepr;
    //standard bit types that can contain 0..2^BITS
    // we might also want to a variant which can contain 0..=MAX which is used for some peripherals like AUX_MU_STAT_REG on bcm2711
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct B8<const BITS: u8>(u8);
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct B16<const BITS: u8>(u16);
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct B32<const BITS: u8>(u32);
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct B64<const BITS: u8>(u64);
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct B128<const BITS: u8>(u128);

    impl<const BITS: u8> From<B8<BITS>> for u8 {
        fn from(value: B8<BITS>) -> Self {
            value.0
        }
    }

    impl<const BITS: u8> From<B8<BITS>> for u16 {
        fn from(value: B8<BITS>) -> Self {
            value.0 as u16
        }
    }

    impl<const BITS: u8> From<B8<BITS>> for u32 {
        fn from(value: B8<BITS>) -> Self {
            value.0 as u32
        }
    }

    impl<const BITS: u8> From<B8<BITS>> for u64 {
        fn from(value: B8<BITS>) -> Self {
            value.0 as u64
        }
    }
    impl<const BITS: u8> From<B8<BITS>> for u128 {
        fn from(value: B8<BITS>) -> Self {
            value.0 as u128
        }
    }

    // impl<const BITS: u8> FromBits<u8> for B8<BITS> {
    //     fn from_bits(value: u8) -> Self {
    //         Self::trimmed_new(value)
    //     }
    // }

    impl<const BITS: u8> From<B16<BITS>> for u16 {
        fn from(value: B16<BITS>) -> Self {
            value.0
        }
    }
    impl<const BITS: u8> From<B16<BITS>> for u32 {
        fn from(value: B16<BITS>) -> Self {
            value.0 as u32
        }
    }
    impl<const BITS: u8> From<B16<BITS>> for u64 {
        fn from(value: B16<BITS>) -> Self {
            value.0 as u64
        }
    }
    impl<const BITS: u8> From<B16<BITS>> for u128 {
        fn from(value: B16<BITS>) -> Self {
            value.0 as u128
        }
    }
    impl<const BITS: u8> From<B32<BITS>> for u32 {
        fn from(value: B32<BITS>) -> Self {
            value.0
        }
    }
    impl<const BITS: u8> From<B32<BITS>> for u64 {
        fn from(value: B32<BITS>) -> Self {
            value.0 as u64
        }
    }
    impl<const BITS: u8> From<B32<BITS>> for u128 {
        fn from(value: B32<BITS>) -> Self {
            value.0 as u128
        }
    }
    impl<const BITS: u8> From<B64<BITS>> for u64 {
        fn from(value: B64<BITS>) -> Self {
            value.0
        }
    }
    impl<const BITS: u8> From<B64<BITS>> for u128 {
        fn from(value: B64<BITS>) -> Self {
            value.0 as u128
        }
    }
    impl<const BITS: u8> From<B128<BITS>> for u128 {
        fn from(value: B128<BITS>) -> Self {
            value.0
        }
    }
    impl<const BITS: u8> Deref for B8<BITS> {
        type Target = u8;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<const BITS: u8> Deref for B16<BITS> {
        type Target = u16;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<const BITS: u8> Deref for B32<BITS> {
        type Target = u32;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<const BITS: u8> Deref for B64<BITS> {
        type Target = u64;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<const BITS: u8> Deref for B128<BITS> {
        type Target = u128;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<const BITS: u8> B8<BITS> {
        pub fn inner(&self) -> u8 {
            self.0
        }
        pub const fn try_new(value: u8) -> Result<Self, u8> {
            if BITS == 0 || BITS >= 8 {
                return Err(value);
            }
            let max = (1u8 << BITS) - 1;
            if value > max {
                return Err(value);
            }
            Ok(Self(value))
        }
        pub const fn checked_new(value: u8) -> Self {
            match Self::try_new(value) {
                Ok(v) => v,
                Err(_) => panic!("Value is out of bounds"),
            }
        }
        pub const fn trimmed_new(value: u8) -> Self {
            let mask = const {
                if BITS == 0 || BITS >= 8 {
                    panic!("Invalid number of bits");
                } else {
                    (1u8 << BITS) - 1
                }
            };
            Self(value & mask)
        }
    }

    impl<const BITS: u8> B16<BITS> {
        pub const fn try_new(value: u16) -> Result<Self, u16> {
            if BITS == 0 || BITS >= 16 {
                return Err(value);
            }
            let max = (1u16 << BITS) - 1;
            if value > max {
                return Err(value);
            }
            Ok(Self(value))
        }
        pub const fn checked_new(value: u16) -> Self {
            match Self::try_new(value) {
                Ok(v) => v,
                Err(_) => panic!("Value is out of bounds"),
            }
        }
        pub const fn trimmed_new(value: u16) -> Self {
            let mask = const {
                if BITS == 0 || BITS >= 16 {
                    panic!("Invalid number of bits");
                } else {
                    (1u16 << BITS) - 1
                }
            };
            Self(value & mask)
        }
    }

    impl<const BITS: u8> B32<BITS> {
        pub const fn try_new(value: u32) -> Result<Self, u32> {
            if BITS == 0 || BITS >= 32 {
                return Err(value);
            }
            let max = (1u32 << BITS) - 1;
            if value > max {
                return Err(value);
            }
            Ok(Self(value))
        }
        pub const fn checked_new(value: u32) -> Self {
            match Self::try_new(value) {
                Ok(v) => v,
                Err(_) => panic!("Value is out of bounds"),
            }
        }
        pub const fn trimmed_new(value: u32) -> Self {
            let mask = const {
                if BITS == 0 || BITS >= 32 {
                    panic!("Invalid number of bits");
                } else {
                    (1u32 << BITS) - 1
                }
            };
            Self(value & mask)
        }
    }
    impl<const BITS: u8> B64<BITS> {
        pub const fn try_new(value: u64) -> Result<Self, u64> {
            if BITS == 0 || BITS >= 64 {
                return Err(value);
            }
            let max = (1u64 << BITS) - 1;
            if value > max {
                return Err(value);
            }
            Ok(Self(value))
        }
        pub const fn checked_new(value: u64) -> Self {
            match Self::try_new(value) {
                Ok(v) => v,
                Err(_) => panic!("Value is out of bounds"),
            }
        }

        pub const fn trimmed_new(value: u64) -> Self {
            let mask = const {
                if BITS == 0 || BITS >= 64 {
                    panic!("Invalid number of bits");
                } else {
                    (1u64 << BITS) - 1
                }
            };
            Self(value & mask)
        }
    }
    impl<const BITS: u8> B128<BITS> {
        pub const fn try_new(value: u128) -> Result<Self, u128> {
            if BITS == 0 || BITS >= 128 {
                return Err(value);
            }
            let max = (1u128 << BITS) - 1;
            if value > max {
                return Err(value);
            }
            Ok(Self(value))
        }
        pub const fn checked_new(value: u128) -> Self {
            match Self::try_new(value) {
                Ok(v) => v,
                Err(_) => panic!("Value is out of bounds"),
            }
        }
        pub const fn trimmed_new(value: u128) -> Self {
            let mask = const {
                if BITS == 0 || BITS >= 128 {
                    panic!("Invalid number of bits");
                } else {
                    (1u128 << BITS) - 1
                }
            };
            Self(value & mask)
        }
    }

    impl<const BITS: u8> BitRepr for B8<BITS> {
        type BitRepr = Self;
    }
    impl<const BITS: u8> BitRepr for B16<BITS> {
        type BitRepr = Self;
    }
    impl<const BITS: u8> BitRepr for B32<BITS> {
        type BitRepr = Self;
    }
    impl<const BITS: u8> BitRepr for B64<BITS> {
        type BitRepr = Self;
    }
    impl<const BITS: u8> BitRepr for B128<BITS> {
        type BitRepr = Self;
    }

    // impl<const BITS: u8> TryFromBits<u8> for B8<BITS> {
    //     fn try_from_bits(bits: u8) -> Result<Self, u8> {
    //         Self::try_new(bits)
    //     }
    // }

    // impl<const BITS: u8> TryFromBits<u16> for B16<BITS> {
    //     fn try_from_bits(bits: u16) -> Result<Self, u16> {
    //         Self::try_new(bits)
    //     }
    // }
    // impl<const BITS: u8> TryFromBits<u32> for B32<BITS> {
    //     fn try_from_bits(bits: u32) -> Result<Self, u32> {
    //         Self::try_new(bits)
    //     }
    // }
    // impl<const BITS: u8> TryFromBits<u64> for B64<BITS> {
    //     fn try_from_bits(bits: u64) -> Result<Self, u64> {
    //         Self::try_new(bits)
    //     }
    // }
    // impl<const BITS: u8> TryFromBits<u128> for B128<BITS> {
    //     fn try_from_bits(bits: u128) -> Result<Self, u128> {
    //         Self::try_new(bits)
    //     }
    // }
}
use implementation::*;
pub type u1 = B8<1>;
pub type u2 = B8<2>;
pub type u3 = B8<3>;
pub type u4 = B8<4>;
pub type u5 = B8<5>;
pub type u6 = B8<6>;
pub type u7 = B8<7>;
pub type u8 = B8<8>;
pub type u9 = B16<9>;
pub type u10 = B16<10>;
pub type u11 = B16<11>;
pub type u12 = B16<12>;
pub type u13 = B16<13>;
pub type u14 = B16<14>;
pub type u15 = B16<15>;
// pub type u16 = B16<16>;
pub type u17 = B32<17>;
pub type u18 = B32<18>;
pub type u19 = B32<19>;
pub type u20 = B32<20>;
pub type u21 = B32<21>;
pub type u22 = B32<22>;
pub type u23 = B32<23>;
pub type u24 = B32<24>;
pub type u25 = B32<25>;
pub type u26 = B32<26>;
pub type u27 = B32<27>;
pub type u28 = B32<28>;
pub type u29 = B32<29>;
pub type u30 = B32<30>;
pub type u31 = B32<31>;
// pub type u32 = B32<32>;
pub type u33 = B64<33>;
pub type u34 = B64<34>;
pub type u35 = B64<35>;
pub type u36 = B64<36>;
pub type u37 = B64<37>;
pub type u38 = B64<38>;
pub type u39 = B64<39>;
pub type u40 = B64<40>;
pub type u41 = B64<41>;
pub type u42 = B64<42>;
pub type u43 = B64<43>;
pub type u44 = B64<44>;
pub type u45 = B64<45>;
pub type u46 = B64<46>;
pub type u47 = B64<47>;
pub type u48 = B64<48>;
pub type u49 = B64<49>;
pub type u50 = B64<50>;
pub type u51 = B64<51>;
pub type u52 = B64<52>;
pub type u53 = B64<53>;
pub type u54 = B64<54>;
pub type u55 = B64<55>;
pub type u56 = B64<56>;
pub type u57 = B64<57>;
pub type u58 = B64<58>;
pub type u59 = B64<59>;
pub type u60 = B64<60>;
pub type u61 = B64<61>;
pub type u62 = B64<62>;
pub type u63 = B64<63>;
// pub type u64 = B64<64>;
pub type u65 = B128<65>;
pub type u66 = B128<66>;
pub type u67 = B128<67>;
pub type u68 = B128<68>;
pub type u69 = B128<69>;
pub type u70 = B128<70>;
pub type u71 = B128<71>;
pub type u72 = B128<72>;
pub type u73 = B128<73>;
pub type u74 = B128<74>;
pub type u75 = B128<75>;
pub type u76 = B128<76>;
pub type u77 = B128<77>;
pub type u78 = B128<78>;
pub type u79 = B128<79>;
pub type u80 = B128<80>;
pub type u81 = B128<81>;
pub type u82 = B128<82>;
pub type u83 = B128<83>;
pub type u84 = B128<84>;
pub type u85 = B128<85>;
pub type u86 = B128<86>;
pub type u87 = B128<87>;
pub type u88 = B128<88>;
pub type u89 = B128<89>;
pub type u90 = B128<90>;
pub type u91 = B128<91>;
pub type u92 = B128<92>;
pub type u93 = B128<93>;
pub type u94 = B128<94>;
pub type u95 = B128<95>;
pub type u96 = B128<96>;
pub type u97 = B128<97>;
pub type u98 = B128<98>;
pub type u99 = B128<99>;
pub type u100 = B128<100>;
pub type u101 = B128<101>;
pub type u102 = B128<102>;
pub type u103 = B128<103>;
pub type u104 = B128<104>;
pub type u105 = B128<105>;
pub type u106 = B128<106>;
pub type u107 = B128<107>;
pub type u108 = B128<108>;
pub type u109 = B128<109>;
pub type u110 = B128<110>;
pub type u111 = B128<111>;
pub type u112 = B128<112>;
pub type u113 = B128<113>;
pub type u114 = B128<114>;
pub type u115 = B128<115>;
pub type u116 = B128<116>;
pub type u117 = B128<117>;
pub type u118 = B128<118>;
pub type u119 = B128<119>;
pub type u120 = B128<120>;
pub type u121 = B128<121>;
pub type u122 = B128<122>;
pub type u123 = B128<123>;
pub type u124 = B128<124>;
pub type u125 = B128<125>;
pub type u126 = B128<126>;
pub type u127 = B128<127>;
//pub type u128 = B128<128>;
