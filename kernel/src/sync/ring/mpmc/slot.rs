use core::{
    ops::{BitAnd, BitOr, Shl, Shr},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::sync::atomics::atomic128::AtomicU128;

pub enum Fill {
    Filled,
    Behind,
    Full,
    Stale,
}
pub enum Take<I> {
    Taken(I),
    Behind,
    Empty,
    Stale,
}

pub trait AtomicWord: Sized {
    type Int: Copy
        + Eq
        + From<u32>
        + Shl<u32, Output = Self::Int>
        + Shr<u32, Output = Self::Int>
        + BitAnd<Output = Self::Int>
        + BitOr<Output = Self::Int>
        + PartialOrd;

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

#[repr(transparent)]
pub struct AtomicSlot<W, const LAP_BITS: u32>(W);

impl<W: AtomicWord, const LAP_BITS: u32> AtomicSlot<W, LAP_BITS> {
    pub const EMPTY: Self = Self(W::ZERO);
    pub const TAG_BITS: u32 = {
        assert!(
            LAP_BITS >= 1 && LAP_BITS <= 31,
            "LAP_BITS must be in 1..=31"
        );
        LAP_BITS + 1
    };
    pub const PAYLOAD_BITS: u32 = W::BITS - Self::TAG_BITS;

    const LAP_MASK: u32 = u32::MAX >> (32 - LAP_BITS);
    const MASK: u32 = u32::MAX >> (32 - Self::TAG_BITS);

    #[inline(always)]
    pub fn tag(lap: u32, full: bool) -> W::Int {
        W::Int::from((lap & Self::LAP_MASK) << 1 | full as u32)
    }

    #[inline(always)]
    pub fn tag_of(w: W::Int) -> W::Int {
        w & W::Int::from(Self::MASK)
    }

    #[inline(always)]
    pub fn pack(payload: W::Int, tag: W::Int) -> W::Int {
        (payload << Self::TAG_BITS) | (tag & Self::TAG_BITS.into())
    }

    #[inline(always)]
    pub fn payload_of(w: W::Int) -> W::Int {
        w >> Self::TAG_BITS
    }

    pub fn try_fill(&self, lap: u32, payload: W::Int) -> Fill {
        let empty = Self::tag(lap, false);
        let full = Self::pack(payload, Self::tag(lap, true));

        let Err(word) = self
            .0
            .compare_exchange(empty, full, Ordering::Release, Ordering::Relaxed)
        else {
            return Fill::Filled;
        };

        let tag = Self::tag_of(word);
        let next = lap.wrapping_add(1);

        if tag == Self::tag(lap, true) || tag == Self::tag(next, false) {
            Fill::Behind
        } else if tag == Self::tag(lap.wrapping_sub(1), true) {
            Fill::Full
        } else {
            Fill::Stale
        }
    }

    pub fn try_take(&self, lap: u32) -> Take<W::Int> {
        let word = self.0.load(Ordering::Acquire);
        let tag = Self::tag_of(word);
        let next = lap.wrapping_add(1);

        if tag == Self::tag(lap, true) {
            match self.0.compare_exchange(
                word,
                Self::tag(next, false),
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => Take::Taken(Self::payload_of(word)),
                Err(_) => Take::Stale,
            }
        } else if tag == Self::tag(next, false) || tag == Self::tag(next, true) {
            Take::Behind
        } else if tag == Self::tag(lap, false) {
            Take::Empty
        } else {
            Take::Stale
        }
    }
}
