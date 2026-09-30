use core::ops::BitOr;

use crate::sync::ring::mpmc::slot::{AtomicSlot64, AtomicSlot128, AtomicSlotEncoding};

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Flags(u8);

impl Flags {
    pub const NONE: Self = Self(0);

    pub const LAZY_PUSH: Self = Self(1 << 0);
    pub const LAZY_POP: Self = Self(1 << 1);

    pub const fn from(val: u8) -> Self {
        Self(val)
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn contains(self, other: Flags) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn union(self, other: Flags) -> Self {
        Self(self.0 | other.0)
    }
}

impl BitOr for Flags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

pub trait AtomicWidth {
    type Atomic: AtomicSlotEncoding;
    const BITS: u32;
}

pub enum U64 {}
pub enum U128 {}

impl AtomicWidth for U64 {
    type Atomic = AtomicSlot64;

    const BITS: u32 = Self::Atomic::BITS;
}

impl AtomicWidth for U128 {
    type Atomic = AtomicSlot128;

    const BITS: u32 = Self::Atomic::BITS;
}
