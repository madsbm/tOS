use core::{
    marker::PhantomData,
    ops::{BitAnd, BitOr, Shl, Shr},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::sync::{
    atomics::atomic128::AtomicU128,
    ring::mpmc::slot::{AtomicSlot64, AtomicSlot128, AtomicSlotEncoding},
};

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

pub trait AtomicWord: Sized {
    type Int: Copy
        + Eq
        + From<u32>
        + Shl<u32, Output = Self::Int>
        + Shr<u32, Output = Self::Int>
        + BitAnd<Output = Self::Int>
        + BitOr<Output = Self::Int>;

    const BITS: u32;
    const ZERO: Self;

    fn load(&self, o: Ordering) -> Self::Int;
    fn compare_exchange(
        &self,
        old: Self::Int,
        new: Self::Int,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Int, Self::Int>;
}

macro_rules! impl_word {
    ($atomic:ty, $int:ty) => {
        impl AtomicWord for $atomic {
            type Int = $int;
            const BITS: u32 = <$int>::BITS;
            const ZERO: Self = <$atomic>::new(0);

            #[inline(always)]
            fn load(&self, o: Ordering) -> $int {
                <$atomic>::load(self, o)
            }

            fn compare_exchange(
                &self,
                old: Self::Int,
                new: Self::Int,
                success: Ordering,
                failure: Ordering,
            ) -> Result<Self::Int, Self::Int> {
                <$atomic>::compare_exchange(self, old, new, success, failure)
            }
        }
    };
}

impl_word!(AtomicU128, u128);
impl_word!(AtomicU64, u64);

pub struct Tag<W, const LAP_BITS: u32>(PhantomData<W>);

impl<W: AtomicWord, const LAP_BITS: u32> Tag<W, LAP_BITS> {
    pub const BITS: u32 = {
        assert!(
            LAP_BITS >= 1 && LAP_BITS <= 31,
            "LAP_BITS must be in 1..=31"
        );
        LAP_BITS + 1
    };

    pub const PAYLOAD_BITS: u32 = W::BITS - Self::BITS;

    const LAP_MASK: u32 = u32::MAX >> (32 - LAP_BITS);
    const MASK: u32 = u32::MAX >> (32 - Self::BITS);

    #[inline(always)]
    pub fn new(lap: u32, full: bool) -> W::Int {
        W::Int::from((lap & Self::LAP_MASK) << 1 | full as u32)
    }

    #[inline(always)]
    pub fn of(w: W::Int) -> W::Int {
        w & W::Int::from(Self::MASK)
    }

    #[inline(always)]
    pub fn pack(payload: W::Int, tag: W::Int) -> W::Int {
        (payload << Self::BITS) | tag
    }

    #[inline(always)]
    pub fn payload(w: W::Int) -> W::Int {
        w >> Self::BITS
    }
}
