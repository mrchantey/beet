use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;


#[derive(Debug, Default)]
pub struct IdIncr(AtomicU64);

impl std::fmt::Display for IdIncr {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "IdIncr({})", self.0.load(Ordering::SeqCst))
	}
}

impl IdIncr {
	pub fn new() -> Self { Self(AtomicU64::new(0)) }
	pub fn next(&self) -> u64 { self.0.fetch_add(1, Ordering::SeqCst) }
}
