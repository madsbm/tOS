use divan::{Bencher, black_box};
use primitives::sync::{
    ring::mpmc::{LazyUnorderedMpmcRing, MpmcRing},
    spinlock::SpinLock,
};

fn main() {
    divan::main()
}

#[divan::bench]
fn spinlock_uncontended(b: Bencher) {
    let lock = SpinLock::new(0u64);
    b.bench(|| *lock.lock() += 1);
}

#[divan::bench(threads = [1, 2, 4, 8])]
fn spinlock_contended(b: Bencher) {
    let lock = SpinLock::new(0u64);
    b.bench(|| *lock.lock() += 1);
}
