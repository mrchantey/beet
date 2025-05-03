#[cfg(feature = "bevy")]
use bevy::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand::distributions::DistIter;
use rand::distributions::Standard;
use rand::distributions::uniform::SampleRange;
use rand::distributions::uniform::SampleUniform;
use rand::prelude::Distribution;
use rand_chacha::ChaCha8Rng;
/// A simple random source, by default retrieved from entropy.
///
/// Enable the `bevy` feature to derive [Resource](bevy::prelude::Resource)
/// ```rust
/// # use bevy::prelude::*;
/// # use sweet_utils::prelude::*;
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
#[cfg_attr(feature = "bevy", derive(Deref, DerefMut, Resource))]
pub struct RandomSource(pub ChaCha8Rng);

impl RandomSource {
	pub fn from_seed(seed: u64) -> Self {
		let rng = ChaCha8Rng::seed_from_u64(seed);
		Self(rng)
	}
}

impl Default for RandomSource {
	fn default() -> Self {
		// let rng = ChaCha8Rng::from_rng(&mut rand::rng());
		let rng = ChaCha8Rng::from_rng(&mut rand::thread_rng()).unwrap();
		Self(rng)
	}
}


/// save the `use rand::Rng` shenannigans
impl RandomSource {
	/// see [Rng::random]
	pub fn random<T>(&mut self) -> T
	where
		Standard: Distribution<T>,
	{
		// self.0.random() TODO update when bevy updates
		self.0.r#gen()
	}

	/// see [Rng::random_iter]
	// pub fn random_iter<T>(
	// 	self,
	// ) -> rand::distr::Iter<rand::distr::StandardUniform, ChaCha8Rng, T>
	// where
	// 	Self: Sized,
	// 	rand::distr::StandardUniform: rand::prelude::Distribution<T>,
	// {
	// 	self.0.random_iter()
	// }

	/// see [Rng::random_range]
	pub fn random_range<T, R>(&mut self, range: R) -> T
	where
		T: SampleUniform,
		R: SampleRange<T>,
	{
		self.0.gen_range(range)
		// self.0.random_range(range)
	}

	/// see [Rng::random_bool]
	pub fn random_bool(&mut self, p: f64) -> bool { self.0.gen_bool(p) }
	// pub fn random_bool(&mut self, p: f64) -> bool { self.0.random_bool(p) }

	/// see [Rng::random_ratio]
	pub fn random_ratio(&mut self, numerator: u32, denominator: u32) -> bool {
		self.0.gen_ratio(numerator, denominator)
		// self.0.random_ratio(numerator, denominator)
	}

	/// see [Rng::sample]
	pub fn sample<T, D: Distribution<T>>(&mut self, distr: D) -> T {
		self.0.sample(distr)
	}

	/// see [Rng::sample_iter]
	pub fn sample_iter<T, D>(self, distr: D) -> DistIter<D, ChaCha8Rng, T>
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

	#[test]
	fn seed() {
		let mut source = RandomSource::from_seed(7);
		let val = source.random_range(10..100);
		assert_eq!(val, 22);
	}

	#[test]
	fn entropy() {
		let mut source = RandomSource::default();
		let val = source.random_range(10..100);
		assert!(val >= 10);
		assert!(val < 100);
	}
}
