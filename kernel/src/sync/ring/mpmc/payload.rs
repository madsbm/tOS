pub trait Payload<I>: Sized {
    const BITS: u32 = (size_of::<Self>() * 8) as u32;

    fn encode(self) -> I;
    fn decode(bits: I) -> Self;
}
