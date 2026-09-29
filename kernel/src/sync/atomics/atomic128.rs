use core::{cell::UnsafeCell, sync::atomic::Ordering};

use crate::sync::atomics::{atomic_cmpxchg, atomic_load, atomic_store};

// The Atomic{U,I}128 hasnt yet been stabilized,
// thus we're rolling our own (as that's always a good idea!).

pub struct AtomicU128 {
    v: UnsafeCell<u128>,
}

impl AtomicU128 {
    pub const fn new(value: u128) -> Self {
        Self {
            v: UnsafeCell::new(value),
        }
    }

    pub fn load(&self, order: Ordering) -> u128 {
        unsafe { atomic_load(self.v.get(), order) }
    }

    pub fn store(&self, val: u128, order: Ordering) {
        unsafe { atomic_store(self.v.get(), val, order) }
    }

    pub fn compare_exchange(
        &self,
        old: u128,
        new: u128,
        success: Ordering,
        failure: Ordering,
    ) -> Result<u128, u128> {
        unsafe { atomic_cmpxchg(self.v.get(), old, new, success, failure) }
    }
}
