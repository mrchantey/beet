use crate::prelude::*;
use rand::RngExt;
use rand::SeedableRng;
use rand::distr::Distribution;
use rand::distr::StandardUniform;
use rand::distr::uniform::SampleRange;
use rand::distr::uniform::SampleUniform;
use rand::rngs::Xoshiro256PlusPlus;

/// Fast, reproducible randomness for simulation: games, steering, training,
/// jitter. Deliberately NOT secure: [`Xoshiro256PlusPlus`] is a few
/// instructions a draw, and its whole state follows from a handful of
/// outputs, so nothing secret may ever come from it. Tokens, keys and
/// passwords come from [`SecureRandom`].
///
/// Seeded from entropy by default, or from a seed for a run that repeats
/// exactly on every target, the algorithm pinned rather than `SmallRng`'s,
/// which differs between 32 and 64 bit platforms.
///
/// ```rust
/// # use beet_core::prelude::*;
/// // seeded from entropy
/// let mut random = FastRandom::default();
/// // or from a seed, the same sequence on every run
/// let mut random = FastRandom::from_seed(7);
/// App::new()
/// 	.insert_resource(random)
/// 	.add_systems(Update, roll);
///
/// fn roll(mut random: ResMut<FastRandom>) {
/// 	let roll = random.random_range(1..=6);
/// 	# assert!((1..=6).contains(&roll));
/// }
/// ```
#[derive(Debug, Clone, Deref, DerefMut, Resource)]
pub struct FastRandom(pub Xoshiro256PlusPlus);

impl FastRandom {
	/// A source seeded with `seed`, the same sequence on every run.
	pub fn from_seed(seed: u64) -> Self {
		Self(Xoshiro256PlusPlus::seed_from_u64(seed))
	}
}

impl Default for FastRandom {
	fn default() -> Self {
		// std seeds from the thread-local entropy generator, no_std (eg bare
		// embedded) from the platform entropy source via getrandom's `sys_rng`.
		// On targets without a built-in getrandom backend the downstream adapter
		// must supply one (eg the esp32 custom backend over the hardware RNG).
		Self(rand::make_rng())
	}
}

/// save the `use rand::RngExt` shenanigans
impl FastRandom {
	/// see [RngExt::random]
	pub fn random<T>(&mut self) -> T
	where
		StandardUniform: Distribution<T>,
	{
		self.0.random()
	}

	/// see [RngExt::random_iter]
	pub fn random_iter<T>(
		self,
	) -> rand::distr::Iter<StandardUniform, Xoshiro256PlusPlus, T>
	where
		Self: Sized,
		StandardUniform: Distribution<T>,
	{
		self.0.random_iter()
	}

	/// see [RngExt::random_range]
	pub fn random_range<T, R>(&mut self, range: R) -> T
	where
		T: SampleUniform,
		R: SampleRange<T>,
	{
		self.0.random_range(range)
	}

	/// see [RngExt::random_bool]
	pub fn random_bool(&mut self, p: f64) -> bool { self.0.random_bool(p) }

	/// see [RngExt::random_ratio]
	pub fn random_ratio(&mut self, numerator: u32, denominator: u32) -> bool {
		self.0.random_ratio(numerator, denominator)
	}

	/// see [RngExt::sample]
	pub fn sample<T, D: Distribution<T>>(&mut self, distr: D) -> T {
		self.0.sample(distr)
	}

	/// see [RngExt::sample_iter]
	pub fn sample_iter<T, D>(
		self,
		distr: D,
	) -> rand::distr::Iter<D, Xoshiro256PlusPlus, T>
	where
		D: Distribution<T>,
		Self: Sized,
	{
		self.0.sample_iter(distr)
	}

	/// see [RngExt::fill]
	pub fn fill<T: rand::Fill>(&mut self, dest: &mut [T]) { self.0.fill(dest) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A seed repeats its sequence exactly, the value pinned so a change of
	/// algorithm, which would shift every seeded run, cannot pass unseen.
	#[crate::test]
	fn seed() { FastRandom::from_seed(7).random_range(10..100).xpect_eq(14); }

	#[crate::test]
	fn entropy() {
		let mut random = FastRandom::default();
		let val = random.random_range(10..100);
		(val >= 10).xpect_true();
		(val < 100).xpect_true();
	}
}
