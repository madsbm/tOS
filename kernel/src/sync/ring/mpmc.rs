use core::{
    hint,
    marker::PhantomData,
    ops::BitOr,
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
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

    fn payload(state: Self::Int) -> Self::Int;
    fn seq(state: Self::Int) -> u32;
    fn ready(state: Self::Int) -> bool;
}

impl<A: AtomicSlotEncoding> Slot for A {
    fn ready(&self) -> bool {
        A::ready(self.load(Ordering::Acquire))
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

    fn payload(state: Self::Int) -> Self::Int {
        todo!()
    }

    fn seq(state: Self::Int) -> u32 {
        todo!()
    }

    fn ready(state: Self::Int) -> bool {
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

    fn payload(state: Self::Int) -> Self::Int {
        todo!()
    }

    fn seq(state: Self::Int) -> u32 {
        todo!()
    }

    fn ready(state: Self::Int) -> bool {
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

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Flags(u8);

impl Flags {
    pub const NONE: Self = Self(0);

    pub const USE_DWCAS: Self = Self(1 << 0);
    pub const LAZY_PUSH: Self = Self(1 << 1);
    pub const LAZY_POP: Self = Self(1 << 2);

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

pub struct MpmcRing<S: AtomicSlotEncoding, T, const N: usize, const FLAGS: u8> {
    ring: Ring<S, T, N>,
    read_idx: AtomicUsize,
    write_idx: AtomicUsize,
}

impl<T, const N: usize, const FLAGS: u8> MpmcRing<AtomicSlot64, T, N, FLAGS> {
    pub const fn new() -> Self {
        Self {
            ring: Ring::<AtomicSlot64, T, N>::new(),
            read_idx: AtomicUsize::new(0),
            write_idx: AtomicUsize::new(0),
        }
    }
}

impl<T, const N: usize, const FLAGS: u8> MpmcRing<AtomicSlot128, T, N, FLAGS> {
    pub const fn new() -> Self {
        Self {
            ring: Ring::<AtomicSlot128, T, N>::new(),
            read_idx: AtomicUsize::new(0),
            write_idx: AtomicUsize::new(0),
        }
    }
}

impl<S: AtomicSlotEncoding, T, const N: usize, const FLAGS: u8> MpmcRing<S, T, N, FLAGS> {
    pub const LOG_CAP: usize = {
        assert!(N.count_ones() == 1, "N must be a power of two!");
        N.trailing_zeros() as usize
    };
    pub const STATE_BITS: usize = Self::LOG_CAP + 1;
    pub const PAYLOAD_BITS: usize = Self::slot_size() - Self::STATE_BITS;

    pub const fn flags() -> Flags {
        Flags(FLAGS)
    }

    pub const fn uses_dwcas() -> bool {
        Self::flags().contains(Flags::USE_DWCAS)
    }

    pub const fn lazy_push() -> bool {
        Self::flags().contains(Flags::LAZY_PUSH)
    }

    pub const fn lazy_pop() -> bool {
        Self::flags().contains(Flags::LAZY_POP)
    }

    pub const fn slot_size() -> usize {
        // FIXME
        if const { Self::uses_dwcas() } {
            128
        } else {
            64
        }
    }

    pub fn try_push(&self, value: T) -> Result<(), T> {
        loop {
            let pos = self.write_idx.load(Ordering::Relaxed);
            let slot = &self.ring[pos];

            let slot_state = slot.state();
            // Slot doesnt hold data (LSB = 0) and FIFO matches
            if slot_state == (pos << 1) as u32 {
                // FIXME
                let old = S::pack(0, pos, false);
                let old = S::pack(value, pos, true);

                match unsafe {
                    slot.packed
                        .compare_exchange(old, new, Ordering::SeqCst, Ordering::SeqCst)
                } {
                    Err(_) => continue,
                    Ok(_) => {
                        if const { !Self::lazy_push() } {
                            let _ = self.write_idx.compare_exchange(
                                pos,
                                pos + 1,
                                Ordering::Release,
                                Ordering::Relaxed,
                            );
                        }

                        return Ok(());
                    }
                }
            // Case A: help the thread who got the spot with advancing the write idx
            // Case B: this slot was already produced and consumed before we saw it, but we can help advance the write idx.
            } else if (slot_state == ((pos << 1) | 1usize) as u32)
                || (slot_state == ((pos + N) << 1) as u32)
            {
                let _ = self.write_idx.compare_exchange(
                    pos,
                    pos + 1,
                    Ordering::Release,
                    Ordering::Relaxed,
                );
            // Ringbuffer is full, we cant overwrite :,(
            } else if slot_state == (((pos - N) << 1) | 1usize) as u32 {
                return Err(value);
            }
        }
    }

    pub fn spin_push(&self, mut value: T) {
        while let Err(val) = self.try_push(value) {
            value = val;
            hint::spin_loop();
        }
    }

    pub fn poll(&self) -> Option<T> {
        unimplemented!();
    }
}
