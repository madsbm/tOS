use crate::sync::ring::mpmc::config::AtomicWidth;

pub trait Payload<T, W: AtomicWidth<N>, const N: usize> {
    fn to_int(value: T) -> W::Int;
    fn from_int(int: W::Int) -> T;
}
