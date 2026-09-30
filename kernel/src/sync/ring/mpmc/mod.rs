use core::{
    hint,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::sync::ring::{
    Ring,
    mpmc::{
        config::{AtomicWidth, Flags, U64},
        slot::AtomicSlotEncoding,
    },
};

pub mod config;
pub mod slot;

pub struct MpmcRing<T, const N: usize, const FLAGS: u8, W: AtomicWidth = U64> {
    ring: Ring<W::Atomic, T, N>,
    read_idx: AtomicUsize,
    write_idx: AtomicUsize,
}

impl<T, const N: usize, const FLAGS: u8, W: AtomicWidth> MpmcRing<T, N, FLAGS, W> {
    pub const LOG_CAP: u32 = {
        assert!(N.count_ones() == 1, "N must be a power of two!");
        N.trailing_zeros()
    };
    pub const STATE_BITS: u32 = Self::LOG_CAP + 1;
    pub const PAYLOAD_BITS: u32 = {
        let SIZE = W::BITS - Self::STATE_BITS;
        assert!(
            core::mem::size_of::<T>() * 8 <= SIZE as usize,
            "T is too large for the selected atomic slot encoding"
        );

        SIZE
    };

    pub const fn flags() -> Flags {
        Flags::from(FLAGS)
    }

    pub const fn lazy_push() -> bool {
        Self::flags().contains(Flags::LAZY_PUSH)
    }

    pub const fn lazy_pop() -> bool {
        Self::flags().contains(Flags::LAZY_POP)
    }

    pub fn try_push(&self, value: T) -> Result<(), T> {
        loop {
            let pos = self.write_idx.load(Ordering::Relaxed);
            let slot = &self.ring[pos];

            let slot_state = slot.state();
            // Slot doesnt hold data (LSB = 0) and FIFO matches
            if slot_state == (pos << 1) as u32 {
                // TODO: Figure out how to pass the PAYLOAD_BITS and STATE_BITS
                // to the AtomicSlotEncoding, such they can be used as masks.
                let old = W::Atomic::pack(0, pos, false);
                let old = W::Atomic::pack(value, pos, true);

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
