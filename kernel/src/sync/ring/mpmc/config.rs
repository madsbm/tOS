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

pub trait AtomicWidth<const N: usize> {
    type Int: Copy;
    type Atomic: AtomicSlotEncoding<Int = Self::Int>;

    const BITS: u32;
    const LOG_CAP: u32 = {
        assert!(N.count_ones() == 1, "N must be a power of two!");
        N.trailing_zeros()
    };

    // Seq. needs LOG_CAP + 1 to fit 2N generations
    const SEQ_BITS: u32 = Self::LOG_CAP + 1;

    // State is Seq + ready/pub. bit size
    const STATE_BITS: u32 = Self::SEQ_BITS + 1;
    const PAYLOAD_BITS: u32 = Self::BITS - Self::STATE_BITS;

    const STATE_MASK: Self::Int;
    const PAYLOAD_MASK: Self::Int;

    fn pack_state(state: u32) -> Self::Int;
    fn pack(payload: Self::Int, state: u32) -> Self::Int;
}

pub enum U64 {}
pub enum U128 {}

impl<const N: usize> AtomicWidth<N> for U64 {
    type Int = u64;
    type Atomic = AtomicSlot64;

    const BITS: u32 = Self::Atomic::BITS;

    const PAYLOAD_MASK: Self::Int = low_mask_u64(<Self as AtomicWidth<N>>::PAYLOAD_BITS);
    const STATE_MASK: Self::Int = !(<Self as AtomicWidth<N>>::PAYLOAD_MASK);

    fn pack(payload: Self::Int, state: u32) -> Self::Int {
        <Self as AtomicWidth<N>>::pack_state(state)
            | (payload & <Self as AtomicWidth<N>>::PAYLOAD_MASK)
    }

    fn pack_state(state: u32) -> Self::Int {
        (state as Self::Int) << <Self as AtomicWidth<N>>::STATE_BITS
    }
}

impl<const N: usize> AtomicWidth<N> for U128 {
    type Int = u128;
    type Atomic = AtomicSlot128;

    const BITS: u32 = Self::Atomic::BITS;

    const PAYLOAD_MASK: Self::Int = low_mask_u128(<Self as AtomicWidth<N>>::PAYLOAD_BITS);
    const STATE_MASK: Self::Int = !(<Self as AtomicWidth<N>>::PAYLOAD_MASK);

    fn pack(payload: Self::Int, state: u32) -> Self::Int {
        <Self as AtomicWidth<N>>::pack_state(state)
            | (payload & <Self as AtomicWidth<N>>::PAYLOAD_MASK)
    }

    fn pack_state(state: u32) -> Self::Int {
        (state as Self::Int) << <Self as AtomicWidth<N>>::STATE_BITS
    }
}

const fn low_mask_u64(bits: u32) -> u64 {
    if bits == 64 {
        u64::MAX
    } else {
        (1 << bits) - 1
    }
}

const fn low_mask_u128(bits: u32) -> u128 {
    if bits == 64 {
        u128::MAX
    } else {
        (1 << bits) - 1
    }
}
