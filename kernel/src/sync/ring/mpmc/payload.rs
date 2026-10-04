use crate::sync::ring::mpmc::config::AtomicWord;

pub trait Payload<T, W: AtomicWord<N>, const N: usize> {
    fn to_int(value: T) -> W::Int;
    fn from_int(int: W::Int) -> T;
    fn store(buf: &Self, int: W::Int);
}
