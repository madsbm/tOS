use core::{
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::sync::{
    atomics::atomic128::AtomicU128,
    ring::{Ring, Slot},
};

pub trait AtomicSlotEncoding: Sized {
    type Int: Copy;

    const BITS: u32;

    fn load(&self, order: Ordering) -> Self::Int;
    fn compare_exchange(
        &self,
        old: Self::Int,
        new: Self::Int,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Int, Self::Int>;

    fn pack(payload: Self::Int, seq: Self::Int, ready: bool) -> Self::Int;

    fn payload(&self) -> Self::Int;
    fn state(&self) -> u32;
}

impl<A: AtomicSlotEncoding> Slot for A {
    fn ready(&self) -> bool {
        self.state() & 1 == 1
    }

    unsafe fn drop_value(&mut self) {}
}

pub struct AtomicSlot64(AtomicU64);

impl AtomicSlotEncoding for AtomicSlot64 {
    type Int = u64;

    const BITS: u32 = Self::Int::BITS;

    fn load(&self, order: Ordering) -> Self::Int {
        todo!()
    }

    fn compare_exchange(
        &self,
        old: Self::Int,
        new: Self::Int,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Int, Self::Int> {
        todo!()
    }

    fn pack(payload: Self::Int, seq: Self::Int, ready: bool) -> Self::Int {
        todo!()
    }

    fn payload(&self) -> Self::Int {
        todo!()
    }

    fn state(&self) -> u32 {
        todo!()
    }
}

pub struct AtomicSlot128(AtomicU128);

impl AtomicSlotEncoding for AtomicSlot128 {
    type Int = u128;

    const BITS: u32 = Self::Int::BITS;

    fn load(&self, order: Ordering) -> Self::Int {
        todo!()
    }

    fn compare_exchange(
        &self,
        old: Self::Int,
        new: Self::Int,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Int, Self::Int> {
        todo!()
    }

    fn pack(payload: Self::Int, seq: Self::Int, ready: bool) -> Self::Int {
        todo!()
    }

    fn payload(&self) -> Self::Int {
        todo!()
    }

    fn state(&self) -> u32 {
        todo!()
    }
}

impl<T, const N: usize> Ring<AtomicSlot128, T, N> {
    pub const fn new() -> Self {
        Self {
            ring: [const { AtomicSlot128(AtomicU128::new(0)) }; N],
            _marker: PhantomData,
        }
    }
}

impl<T, const N: usize> Ring<AtomicSlot64, T, N> {
    pub const fn new() -> Self {
        Self {
            ring: [const { AtomicSlot64(AtomicU64::new(0)) }; N],
            _marker: PhantomData,
        }
    }
}
