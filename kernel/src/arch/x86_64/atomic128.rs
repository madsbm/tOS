use core::{cell::UnsafeCell, sync::atomic::Ordering};

pub struct AtomicU128 {
    v: UnsafeCell<u128>,
}

impl AtomicU128 {
    pub const fn new(value: u128) -> Self {
        Self {
            v: UnsafeCell::new(value),
        }
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "cmpxchg16b"))]
    pub fn compare_exchange(
        &mut self,
        old: u128,
        new: u128,
        success: Ordering,
        failure: Ordering,
    ) -> Result<u128, u128> {
        let res =
            unsafe { core::arch::x86_64::cmpxchg16b(self.v.get(), old, new, success, failure) };
        if res == old { Ok(res) } else { Err(res) }
    }

    #[cfg(not(all(target_arch = "x86_64", target_feature = "cmpxchg16b")))]
    pub fn compare_exchange(
        &mut self,
        _old: u128,
        _new: u128,
        _success: Ordering,
        _failure: Ordering,
    ) -> Result<u128, u128> {
        panic!("DWCAS (cmpxchg16b) is unsupported with these compilation flags!")
    }
}
