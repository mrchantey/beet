use crate::prelude::*;
use rand::RngExt;
use rand::SeedableRng;
use rand::distr::uniform::SampleRange;
use rand::distr::uniform::SampleUniform;
use rand::rngs::ChaCha20Rng;
use rand::rngs::SysRng;

/// Randomness for secrets: tokens, verifiers, keys and generated passwords.
///
/// Seeded straight from the operating system's entropy source (getrandom,
/// WebCrypto on wasm) into a full-round ChaCha20, and deliberately not
/// seedable, not a `Resource` and not interchangeable with [`RandomSource`],
/// whose reproducible seeds suit a simulation and would make a secret
/// predictable. Construct one where the secret is made, never share it.
///
/// ```rust
/// # use beet_core::prelude::*;
/// let mut random = SecureRandom::new().unwrap();
/// let token: [u8; 32] = random.bytes();
/// let digit = random.random_range(0..10);
/// # assert!(digit < 10 && token.len() == 32);
/// ```
pub struct SecureRandom(ChaCha20Rng);

impl core::fmt::Debug for SecureRandom {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("SecureRandom").finish_non_exhaustive()
	}
}

impl SecureRandom {
	/// A generator seeded from the operating system, failing only where the
	/// platform offers no entropy source.
	pub fn new() -> Result<Self> {
		ChaCha20Rng::try_from_rng(&mut SysRng)?.xmap(Self).xok()
	}

	/// `N` random bytes.
	pub fn bytes<const N: usize>(&mut self) -> [u8; N] { self.0.random() }

	/// A uniform value in `range`, ie an index into an alphabet.
	pub fn random_range<T, R>(&mut self, range: R) -> T
	where
		T: SampleUniform,
		R: SampleRange<T>,
	{
		self.0.random_range(range)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// Two generators never agree, and the state is never printed.
	#[crate::test]
	fn draws_from_the_os() {
		let first: [u8; 32] = SecureRandom::new().unwrap().bytes();
		let second: [u8; 32] = SecureRandom::new().unwrap().bytes();
		(first == second).xpect_false();
		format!("{:?}", SecureRandom::new().unwrap())
			.xpect_eq("SecureRandom { .. }");
	}
}
