use core::{
    intrinsics::{self, AtomicOrdering},
    sync::atomic::Ordering,
};

pub mod atomic128;

#[inline]
unsafe fn atomic_load<T: Copy>(dst: *const T, order: Ordering) -> T {
    unsafe {
        match order {
            Ordering::Relaxed => {
                intrinsics::atomic_load::<T, { AtomicOrdering::Relaxed }, false>(dst)
            }
            Ordering::Acquire => {
                intrinsics::atomic_load::<T, { AtomicOrdering::Acquire }, false>(dst)
            }
            Ordering::SeqCst => {
                intrinsics::atomic_load::<T, { AtomicOrdering::SeqCst }, false>(dst)
            }
            _ => panic!("nonexistent load operation"),
        }
    }
}

#[inline]
unsafe fn atomic_store<T: Copy>(dst: *mut T, val: T, order: Ordering) {
    unsafe {
        match order {
            Ordering::Relaxed => {
                intrinsics::atomic_store::<T, { AtomicOrdering::Relaxed }, false>(dst, val)
            }
            Ordering::Release => {
                intrinsics::atomic_store::<T, { AtomicOrdering::Release }, false>(dst, val)
            }
            Ordering::SeqCst => {
                intrinsics::atomic_store::<T, { AtomicOrdering::SeqCst }, false>(dst, val)
            }
            _ => panic!("nonexistent load operation"),
        }
    }
}

#[inline]
unsafe fn atomic_cmpxchg<T: Copy>(
    dst: *mut T,
    old: T,
    new: T,
    success: Ordering,
    failure: Ordering,
) -> Result<T, T> {
    // This is internal, but it's okay! We can sin - just dont update your compiler too often :)
    unsafe { core::sync::atomic::atomic_compare_exchange(dst, old, new, success, failure) }
}
