use core::fmt;
use std::{hint, thread};

use divan::{Bencher, black_box, counter::ItemsCount};
use primitives::sync::ring::mpmc::{
    BigLazyMpmcRing, BigLazyUnorderedMpmcRing, BigMpmcRing, BigUnorderedMpmcRing, LazyMpmcRing,
    LazyUnorderedMpmcRing, MpmcRing, UnorderedMpmcRing, payload::Payload, slot::AtomicWord,
};

fn main() {
    divan::main();
}

const CAP: usize = 1024;
const MSGS: usize = 1 << 16;

type Strict = MpmcRing<u32, CAP>;
type Lazy = LazyMpmcRing<u32, CAP>;
type Unordered = UnorderedMpmcRing<u32, CAP>;
type LazyUnordered = LazyUnorderedMpmcRing<u32, CAP>;
type BigStrict = BigMpmcRing<u32, CAP>;
type BigLazy = BigLazyMpmcRing<u32, CAP>;
type BigUnordered = BigUnorderedMpmcRing<u32, CAP>;
type BigLazyUnordered = BigLazyUnorderedMpmcRing<u32, CAP>;

trait Ring: Sync {
    fn create() -> Self;
    fn send(&self, v: u32);
    fn recv(&self) -> Option<u32>;
}

impl<const FLAGS: u8, const LAP_BITS: u32, W> Ring for MpmcRing<u32, CAP, FLAGS, LAP_BITS, W>
where
    W: AtomicWord,
    u32: Payload<W::Int>,
{
    fn create() -> Self {
        Self::new()
    }

    fn send(&self, v: u32) {
        self.spin_push(v);
    }

    fn recv(&self) -> Option<u32> {
        self.pop()
    }
}

#[divan::bench(types = [
	Strict, Lazy, Unordered, LazyUnordered,
	BigStrict, BigLazy, BigUnordered, BigLazyUnordered
])]
fn uncontended<R: Ring>(b: Bencher) {
    let ring = R::create();
    b.bench_local(|| {
        ring.send(black_box(1));
        black_box(ring.recv());
    });
}

#[derive(Clone, Copy)]
struct Pc(usize, usize);

impl fmt::Display for Pc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}P/{}C", self.0, self.1)
    }
}

#[divan::bench(
	types = [
		Strict, Lazy,
		BigStrict, BigLazy,
	],
	args = [Pc(1, 1), Pc(1, 4), Pc(4, 1), Pc(4, 4), Pc(1, 8), Pc(8, 1)],
	sample_count = 30
)]
fn throughput<R: Ring>(b: Bencher, Pc(p, c): Pc) {
    let per = (MSGS / p) as u64;
    let expected = p as u64 * per * (per - 1) / 2;

    b.counter(ItemsCount::new(MSGS)).bench(|| {
        let ring = R::create();

        let sum: u64 = thread::scope(|s| {
            for _ in 0..p {
                s.spawn(|| {
                    for i in 0..MSGS / p {
                        ring.send(black_box(i as u32));
                    }
                });
            }

            let consumers: Vec<_> = (0..c)
                .map(|_| {
                    s.spawn(|| {
                        let mut sum = 0u64;
                        let mut n = 0;

                        while n < MSGS / c {
                            match ring.recv() {
                                Some(v) => {
                                    sum += v as u64;
                                    n += 1;
                                }
                                None => hint::spin_loop(),
                            }
                        }

                        sum
                    })
                })
                .collect();

            consumers.into_iter().map(|h| h.join().unwrap()).sum()
        });

        assert_eq!(sum, expected);
    });
}
