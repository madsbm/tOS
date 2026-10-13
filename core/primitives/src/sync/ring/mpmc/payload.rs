use core::ptr::NonNull;

pub trait Payload<I>: Sized {
    const BITS: u32 = (size_of::<Self>() * 8) as u32;

    fn encode(self) -> I;
    fn decode(bits: I) -> Self;
}

macro_rules! impl_payload {
	($word:ty => $($t:ty),*) => {$(
		impl Payload<$word> for $t {
			#[inline(always)]
			fn encode(self) -> $word { self as $word }

			#[inline(always)]
			fn decode(bits: $word) -> Self { bits as Self }
		}
	)*};
}

impl_payload!(u64 => u8, u16, u32, i8, i16, i32);
impl_payload!(u128 => u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

const VIRT_ADDR_BITS: u32 = if cfg!(feature = "la57") { 57 } else { 48 };

macro_rules! impl_payload_ptr {
	($($word:ty),*) => {$(
		impl<T> Payload<$word> for NonNull<T> {
			const BITS: u32 = VIRT_ADDR_BITS - align_of::<T>().trailing_zeros();

			#[inline(always)]
			fn encode(self) -> $word {
				let addr = self.as_ptr() as u64 & ((1 << VIRT_ADDR_BITS) - 1);
				(addr >> align_of::<T>().trailing_zeros()) as $word
			}

			#[inline(always)]
			fn decode(bits: $word) -> Self {
				const SHIFT: u32 = 64 - VIRT_ADDR_BITS;
				let addr = (bits as u64) << align_of::<T>().trailing_zeros();
				let addr = ((addr << SHIFT) as i64 >> SHIFT) as u64;
				unsafe { NonNull::new_unchecked(addr as *mut T) }
			}
		}
	)*};
}

impl_payload_ptr!(u64, u128);
