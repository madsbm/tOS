use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    mem::MaybeUninit,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{
    arch::x86_64::atomics::atomic128::AtomicU128,
    sync::ring::{Ring, Slot},
};

pub unsafe trait QueuePayload: Sized {
    fn into_bytes(self) -> [u8; 12];
    fn from_bytes(bytes: [u8; 12]) -> Self;
}

#[repr(transparent)]
pub struct AtomicSlotEntry(AtomicU128);

impl AtomicSlotEntry {
    pub const fn empty() -> Self {
        Self(AtomicU128::new(0))
    }

    #[inline]
    pub fn load(&self, order: Ordering) -> u128 {
        self.0.load(order)
    }

    #[inline]
    pub fn store(&self, order: Ordering, val: u128) {
        self.0.store(val, order);
    }

    #[inline]
    pub fn ready(&self, load_with: Ordering) -> bool {
        (self.load(load_with) & 1) > 0
    }

    #[inline]
    pub fn seq_num(&self, load_with: Ordering) -> u128 {
        self.load(load_with) >> 1
    }
}

pub struct AtomicSlot<T> {
    pub seq: AtomicSlotEntry,
    pub value: UnsafeCell<MaybeUninit<T>>,
}

impl<T> AtomicSlot<T> {
    pub const fn empty() -> Self {
        Self {
            seq: AtomicSlotEntry::empty(),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }
}

impl<T> Slot<T> for AtomicSlot<T> {}

impl<T, const N: usize> Ring<AtomicSlot<T>, T, N> {
    pub const fn new() -> Self {
        Self {
            ring: [const { AtomicSlot::empty() }; N],
            _marker: PhantomData,
        }
    }
}

pub struct MpmcRing<T, const N: usize> {
    ring: Ring<AtomicSlot<T>, T, N>,
    read_idx: AtomicUsize,
    write_idx: AtomicUsize,
}

impl<T, const N: usize> MpmcRing<T, N> {
    pub const fn new() -> Self {
        Self {
            ring: Ring::<AtomicSlot<T>, T, N>::new(),
            read_idx: AtomicUsize::new(0),
            write_idx: AtomicUsize::new(0),
        }
    }

    pub fn try_push(&self, value: T) -> bool {
        let pos = self.write_idx.load(Ordering::Relaxed);
        let slot = &self.ring[pos];

        let seq = slot.seq.load(Ordering::Acquire);
        if seq == pos as u128 {}

        unimplemented!()
    }

    pub fn poll(&self) -> Option<T> {
        unimplemented!();
    }
}
