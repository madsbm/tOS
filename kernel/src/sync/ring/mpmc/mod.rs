use core::{
    hint,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::sync::ring::{
    Ring,
    mpmc::{
        config::{AtomicWord, Flags, U64},
        payload::Payload,
        slot::AtomicSlotEncoding,
    },
};

pub mod config;
pub mod payload;
pub mod slot;

pub struct MpmcRing<T, const N: usize, const FLAGS: u8, W: AtomicWord<N> = U64> {
    ring: Ring<W::Atomic, T, N>,
    read_idx: AtomicU32,
    write_idx: AtomicU32,
}

// TODO: some compile time assertion that size_of::<T>() * 8 <= W::PAYLOAD_BITS
impl<T: Payload<T, W, N>, const N: usize, const FLAGS: u8, W: AtomicWord<N>>
    MpmcRing<T, N, FLAGS, W>
{
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
        let payload = T::to_int(value);
        let mut pos: u32;

        loop {
            pos = self.write_idx.load(Ordering::Relaxed);
            let slot = &self.ring[pos as usize];

            let slot_state = slot.state();
            // Slot doesnt hold data (LSB = 0) and FIFO matches
            if slot_state == (pos << 1) as u32 {
                // TODO: Figure out how to pass the PAYLOAD_BITS and STATE_BITS
                // to the AtomicSlotEncoding, such they can be used as masks.
                let old = W::pack_state(pos);
                let new = W::pack(payload, pos | 1);

                match slot.compare_exchange(old, new, Ordering::SeqCst, Ordering::SeqCst) {
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
            } else if (slot_state == (pos << 1) | 1u32) || (slot_state == (pos + (N as u32)) << 1) {
                let _ = self.write_idx.compare_exchange(
                    pos,
                    pos + 1,
                    Ordering::Release,
                    Ordering::Relaxed,
                );
            // Ringbuffer is full, we cant overwrite :,(
            } else if slot_state == (pos - (N as u32)) << 1 {
                return Err(T::from_int(payload));
            }
        }
    }

    pub fn spin_push(&self, mut value: T) {
        while let Err(val) = self.try_push(value) {
            value = val;
            hint::spin_loop();
        }
    }

    pub fn pop(&self, buf: &mut T) -> bool {
        let mut pos: u32;

        loop {
            pos = self.read_idx.load(Ordering::Relaxed);
            let slot_atomic = &self.ring[pos as usize];

            let slot_content = slot_atomic.load(Ordering::Acquire);

            let slot_state = W::state(slot_content);
            // Case: slot holds data, and generation matches
            if slot_state == pos | 1 {
                let empty = W::pack_state((pos + N as u32) << 1);

                match slot_atomic.compare_exchange(
                    slot_content,
                    empty,
                    Ordering::Release,
                    Ordering::Relaxed,
                ) {
                    Err(_) => continue,
                    Ok(_) => {
                        T::store(buf, W::payload(slot_content));

                        if const { !Self::lazy_pop() } {
                            let _ = self.read_idx.compare_exchange(
                                pos,
                                pos + 1,
                                Ordering::Release,
                                Ordering::Relaxed,
                            );
                        }

                        return true;
                    }
                }
            }
        }
    }

    pub fn poll(&self) -> Option<T> {
        unimplemented!();
    }
}
