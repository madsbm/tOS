pub mod mpmc;
pub mod mpsc;
pub mod spsc;

pub trait Queue<T> {
    fn push(&self, val: T) -> Result<(), T>;

    fn pop(&self) -> Option<T>;

    fn is_empty(&self) -> Option<T>;
}
