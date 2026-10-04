use core::{
    marker::PhantomData,
    ops::{BitAnd, BitOr, Shl, Shr},
    sync::atomic::{AtomicU64, Ordering},
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
