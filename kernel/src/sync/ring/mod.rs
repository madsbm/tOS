use core::{cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit, ops::Index};

pub mod mpmc;
pub mod mpsc;
pub mod spsc;

pub trait Slot<T> {}

pub struct SimpleSlot<T> {
    value: UnsafeCell<MaybeUninit<T>>,
}

impl<T> SimpleSlot<T> {
    pub const fn empty() -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }
}

impl<T> Slot<T> for SimpleSlot<T> {}

pub struct Ring<S, T, const N: usize>
where
    S: Slot<T>,
{
    ring: [S; N],
    _marker: PhantomData<T>,
}

impl<T, const N: usize> Ring<SimpleSlot<T>, T, N> {
    pub const fn new() -> Self {
        Self {
            ring: [const { SimpleSlot::empty() }; N],
            _marker: PhantomData,
        }
    }
}

impl<S, T, const N: usize> Index<usize> for Ring<S, T, N>
where
    S: Slot<T>,
{
    type Output = S;

    fn index(&self, index: usize) -> &Self::Output {
        &self.ring[index]
    }
}
