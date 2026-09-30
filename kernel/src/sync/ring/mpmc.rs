use core::{
    cell::UnsafeCell,
    hint,
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ops::{BitAnd, BitOr},
    sync::atomic::{AtomicU32, AtomicUsize, Ordering},
};

use crate::sync::{
    atomics::atomic128::AtomicU128,
    ring::{Ring, Slot},
};

#[repr(transparent)]
pub struct AtomicSlotFieldState(AtomicU32);

impl AtomicSlotFieldState {
    pub const fn empty() -> Self {
        Self(AtomicU32::new(0))
    }

    pub const fn from_seq(val: u32) -> Self {
        Self::from(val << 1)
    }

    pub const fn from(val: u32) -> Self {
        Self(AtomicU32::new(val))
    }

    #[inline]
    pub fn load(&self, order: Ordering) -> u32 {
        self.0.load(order)
    }

    #[inline]
    pub fn store(&self, order: Ordering, val: u32) {
        self.0.store(val, order);
    }

    #[inline]
    pub fn ready(&self) -> bool {
        (self.load(Ordering::Acquire) & 1) > 0
    }

    #[inline]
    pub fn seq_num(&self) -> u32 {
        self.load(Ordering::Relaxed) >> 1
    }
}

pub struct AtomicSlotPayload {}

#[repr(C, align(64))]
pub struct AtomicSlotFields<T> {
    state: AtomicSlotFieldState,
    value: UnsafeCell<MaybeUninit<T>>,
}

impl<T> AtomicSlotFields<T> {
    pub const fn new(state: AtomicSlotFieldState, value: T) -> Self {
        Self {
            state,
            value: UnsafeCell::new(MaybeUninit::new(value)),
        }
    }

    pub const fn new_unused(state: AtomicSlotFieldState) -> Self {
        Self {
            state,
            value: UnsafeCell::new(MaybeUninit::zeroed()),
        }
    }

    pub const fn empty_with_seq(seq: u32) -> Self {
        Self {
            state: AtomicSlotFieldState::from_seq(seq),
            value: UnsafeCell::new(MaybeUninit::zeroed()),
        }
    }

    pub fn pack(self) -> u128 {
        unimplemented!()
    }
}

#[repr(C, align(64))]
pub union AtomicSlot<T> {
    packed: ManuallyDrop<AtomicU128>,
    fields: ManuallyDrop<AtomicSlotFields<T>>,
}

impl<T> AtomicSlot<T> {
    pub const fn empty() -> Self {
        Self::empty_with_seq(0)
    }

    pub const fn empty_with_seq(seq: u32) -> Self {
        Self {
            fields: ManuallyDrop::new(AtomicSlotFields::empty_with_seq(seq)),
        }
    }

    #[inline]
    pub fn ready(&self) -> bool {
        unsafe { self.fields.state.ready() }
    }

    #[inline]
    pub fn seq(&self) -> u32 {
        unsafe { self.fields.state.seq_num() }
    }

    pub fn state(&self) -> u32 {
        unsafe { self.fields.state.load(Ordering::Acquire) }
    }
}

impl<T> Slot<T> for AtomicSlot<T> {
    fn ready(&self) -> bool {
        self.ready()
    }

    unsafe fn drop_value(&mut self) {
        if self.ready() {
            unsafe {
                let cell = self.fields.value.get();
                let val_ptr = (*cell).as_mut_ptr();
                core::ptr::drop_in_place(val_ptr);
            }
        }
    }
}

impl<T, const N: usize> Ring<AtomicSlot<T>, T, N> {
    pub const fn new() -> Self {
        Self {
            ring: const { Self::init_ring() },
            _marker: PhantomData,
        }
    }

    const fn init_ring() -> [AtomicSlot<T>; N] {
        let mut ring = [const { AtomicSlot::empty() }; N];

        let mut i = 0;
        while i < N {
            ring[i] = AtomicSlot::empty_with_seq(i as u32);
            i += 1;
        }

        ring
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

pub struct MpmcRing<T, const N: usize, const FLAGS: u8> {
    ring: Ring<AtomicSlot<T>, T, N>,
    read_idx: AtomicUsize,
    write_idx: AtomicUsize,
}

impl<T, const N: usize, const FLAGS: u8> MpmcRing<T, N, FLAGS> {
    pub const LOG_CAP: usize = {
        assert!(N.count_ones() == 1, "N must be a power of two!");
        N.trailing_zeros() as usize
    };
    pub const STATE_BITS: usize = Self::LOG_CAP + 1;
    pub const PAYLOAD_BITS: usize = Self::slot_size() - Self::STATE_BITS;

    pub const fn new() -> Self {
        assert!(N.is_power_of_two());

        Self {
            ring: Ring::<AtomicSlot<T>, T, N>::new(),
            read_idx: AtomicUsize::new(0),
            write_idx: AtomicUsize::new(0),
        }
    }

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
                let old = AtomicSlotFields::<T>::new_unused(AtomicSlotFieldState::from(pos)).pack();
                let new = AtomicSlotFields::<T>::new(AtomicSlotFieldState::from(pos), value).pack();

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
