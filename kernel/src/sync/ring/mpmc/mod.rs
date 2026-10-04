use core::{
    hint,
    marker::PhantomData,
    sync::atomic::{AtomicU32, AtomicU64, Ordering},
};

use crate::sync::ring::mpmc::{
    config::Flags,
    payload::Payload,
    slot::{AtomicSlot, AtomicWord, Fill, Take},
};

pub mod config;
pub mod payload;
pub mod slot;

pub struct MpmcRing<
    T,
    const N: usize,
    const FLAGS: u8,
    const LAP_BITS: u32,
    W: AtomicWord = AtomicU64,
> {
    slots: [AtomicSlot<W, LAP_BITS>; N],
    read_idx: AtomicU32,
    write_idx: AtomicU32,
    _marker: PhantomData<T>,
}

unsafe impl<T: Send, const N: usize, const FLAGS: u8, const LAP_BITS: u32, W: AtomicWord> Send
    for MpmcRing<T, N, FLAGS, LAP_BITS, W>
{
}
unsafe impl<T: Send, const N: usize, const FLAGS: u8, const LAP_BITS: u32, W: AtomicWord> Sync
    for MpmcRing<T, N, FLAGS, LAP_BITS, W>
{
}

impl<T, const N: usize, const FLAGS: u8, const LAP_BITS: u32, W> MpmcRing<T, N, FLAGS, LAP_BITS, W>
where
    T: Payload<W::Int>,
    W: AtomicWord,
{
    const LOG_CAP: u32 = {
        assert!(N.is_power_of_two(), "N must be a power of two!");
        N.trailing_zeros()
    };

    const CHECK: () = {
        assert!(
            LAP_BITS + Self::LOG_CAP <= 32,
            "LAP_BITS + log2(N) must fit into the u32 position counter"
        );
        assert!(
            T::BITS <= AtomicSlot::<W, LAP_BITS>::PAYLOAD_BITS,
            "payload does not fit next to the tag"
        );
    };

    const LAZY_PUSH: bool = Flags::from(FLAGS).contains(Flags::LAZY_PUSH);
    const LAZY_POP: bool = Flags::from(FLAGS).contains(Flags::LAZY_POP);

    pub const fn new() -> Self {
        let () = Self::CHECK;

        Self {
            slots: [const { AtomicSlot::EMPTY }; N],
            read_idx: AtomicU32::new(0),
            write_idx: AtomicU32::new(0),
            _marker: PhantomData,
        }
    }

    fn slot(&self, pos: u32) -> &AtomicSlot<W, LAP_BITS> {
        &self.slots[pos as usize & (N - 1)]
    }

    fn lap(pos: u32) -> u32 {
        pos >> Self::LOG_CAP
    }

    fn try_advance(idx: &AtomicU32, pos: u32) -> bool {
        idx.compare_exchange(
            pos,
            pos.wrapping_add(1),
            Ordering::Release,
            Ordering::Relaxed,
        )
        .is_ok()
    }

    pub fn try_push(&self, value: T) -> Result<(), T> {
        let payload = value.encode();

        loop {
            let pos = self.write_idx.load(Ordering::Relaxed);
            match self.slot(pos).try_fill(Self::lap(pos), payload) {
                Fill::Filled => {
                    if !Self::LAZY_PUSH {
                        Self::try_advance(&self.write_idx, pos);
                    }
                    return Ok(());
                }
                Fill::Behind => {
                    Self::try_advance(&self.write_idx, pos);
                }
                Fill::Full if self.write_idx.load(Ordering::Relaxed) == pos => {
                    return Err(T::decode(payload));
                }
                Fill::Full | Fill::Stale => hint::spin_loop(),
            }
        }
    }

    pub fn spin_push(&self, mut value: T) {
        while let Err(val) = self.try_push(value) {
            value = val;
            hint::spin_loop();
        }
    }

    pub fn pop(&self) -> Option<T> {
        loop {
            let pos = self.read_idx.load(Ordering::Relaxed);
            match self.slot(pos).try_take(Self::lap(pos)) {
                Take::Taken(payload) => {
                    if !Self::LAZY_POP {
                        Self::try_advance(&self.read_idx, pos);
                    }
                    return Some(T::decode(payload));
                }
                Take::Behind => {
                    Self::try_advance(&self.read_idx, pos);
                }
                Take::Empty if self.read_idx.load(Ordering::Relaxed) == pos => return None,
                Take::Empty | Take::Stale => hint::spin_loop(),
            }
        }
    }

    pub fn poll(&self) -> Option<T> {
        unimplemented!();
    }
}
