use crate::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand::distr::Distribution;
use rand::distr::StandardUniform;
use rand::distr::uniform::SampleRange;
use rand::distr::uniform::SampleUniform;
use rand_chacha::ChaCha8Rng;

/// A simple random source, by default retrieved from entropy.
///
/// Enable the `bevy` feature to derive [Resource](bevy::prelude::Resource)
/// ```rust
/// # use beet_core::prelude::*;
/// # use rand::Rng;
///
/// // defaults to from entropy
/// let mut source = RandomSource::default();
/// // or from a seed
/// let mut source = RandomSource::from_seed(7);
/// App::new()
/// 	.insert_resource(source)
/// 	.add_systems(Update,use_source);
///
///
/// fn use_source(mut source: ResMut<RandomSource>) {
/// 	println!("Random number: {}", source.random_range(1..1000));
/// }
/// ```
///https://bevyengine.org/examples/math/random-sampling/
#[derive(Deref, DerefMut, Resource)]
pub struct RandomSource(pub ChaCha8Rng);

impl RandomSource {
	/// Creates a new [`RandomSource`] with the given seed for reproducible randomness.
	pub fn from_seed(seed: u64) -> Self {
		let rng = ChaCha8Rng::seed_from_u64(seed);
		Self(rng)
	}
}

impl Default for RandomSource {
	fn default() -> Self {
		cfg_if! {
			if #[cfg(feature = "std")] {
				// std: seed from the thread-local entropy generator.
				Self(ChaCha8Rng::from_rng(&mut rand::rng()))
			} else {
				// no_std (eg bare embedded): seed from the platform entropy
				// source via getrandom's `os_rng`. On targets without a built-in
				// getrandom backend the downstream adapter must supply one (eg
				// the esp32 custom backend over the hardware RNG).
				Self(ChaCha8Rng::from_os_rng())
			}
		}
	}
}


/// save the `use rand::Rng` shenannigans
impl RandomSource {
	/// see [Rng::random]
	pub fn random<T>(&mut self) -> T
	where
		StandardUniform: Distribution<T>,
	{
		self.0.random()
	}

	/// see [Rng::random_iter]
	pub fn random_iter<T>(
		self,
	) -> rand::distr::Iter<StandardUniform, ChaCha8Rng, T>
	where
		Self: Sized,
		StandardUniform: Distribution<T>,
	{
		self.0.random_iter()
	}

	/// see [Rng::random_range]
	pub fn random_range<T, R>(&mut self, range: R) -> T
	where
		T: SampleUniform,
		R: SampleRange<T>,
	{
		self.0.random_range(range)
	}

	/// see [Rng::random_bool]
	pub fn random_bool(&mut self, p: f64) -> bool { self.0.random_bool(p) }

	/// see [Rng::random_ratio]
	pub fn random_ratio(&mut self, numerator: u32, denominator: u32) -> bool {
		self.0.random_ratio(numerator, denominator)
	}

	/// see [Rng::sample]
	pub fn sample<T, D: Distribution<T>>(&mut self, distr: D) -> T {
		self.0.sample(distr)
	}

	/// see [Rng::sample_iter]
	pub fn sample_iter<T, D>(
		self,
		distr: D,
	) -> rand::distr::Iter<D, ChaCha8Rng, T>
	where
		D: Distribution<T>,
		Self: Sized,
	{
		self.0.sample_iter(distr)
	}

	/// see [Rng::fill]
	pub fn fill<T: rand::Fill + ?Sized>(&mut self, dest: &mut T) {
		self.0.fill(dest)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn seed() {
		let mut source = RandomSource::from_seed(7);
		let val = source.random_range(10..100);
		val.xpect_eq(22);
	}

	#[crate::test]
	fn entropy() {
		let mut source = RandomSource::default();
		let val = source.random_range(10..100);
		(val >= 10).xpect_true();
		(val < 100).xpect_true();
	}
}
