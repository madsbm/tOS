use core::{cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit, ops::Index};

pub mod mpmc;
pub mod mpsc;
pub mod spsc;

pub trait Slot {
    fn ready(&self) -> bool;

    unsafe fn drop_value(&mut self);
}

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

impl<T> Slot for SimpleSlot<T> {
    fn ready(&self) -> bool {
        unimplemented!()
    }

    unsafe fn drop_value(&mut self) {
        unimplemented!()
    }
}

pub struct Ring<S, T, const N: usize>
where
    S: Slot,
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
    S: Slot,
{
    type Output = S;

    fn index(&self, index: usize) -> &Self::Output {
        &self.ring[index]
    }
}

impl<S: Slot, T, const N: usize> Drop for Ring<S, T, N> {
    fn drop(&mut self) {
        for i in 0..N {
            let slot = &mut self.ring[i];
            unsafe { slot.drop_value() };
        }
    }
}

pub trait Queue<T> {
    fn push(&self, val: T) -> Result<(), T>;

    fn pop(&self) -> Option<T>;

    fn is_empty(&self) -> Option<T>;
}
