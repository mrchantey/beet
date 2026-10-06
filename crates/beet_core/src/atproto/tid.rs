use crate::prelude::*;

/// A timestamp identifier, the protocol's conventional rkey for a record
/// nothing else keys: one zero bit, 53 bits of microseconds since the Unix
/// epoch and 10 bits of clock id, written as 13 base32-sortable characters, so
/// the string order is the creation order.
///
/// [`Tid::now`] mints one, collision free without coordination: a process
/// picks its clock id once at random, and its minter is monotonic, using the
/// last mint plus one microsecond when the clock has not moved past it.
/// `Tid::from(Timestamp)` is that instant with the clock bits zero, for an
/// identity that must be a function of a time alone.
///
/// ```
/// # use beet_core::prelude::*;
/// let tid = Tid::parse("3mw72aaeuj22n").unwrap();
/// tid.to_string().xpect_eq("3mw72aaeuj22n");
/// tid.timestamp().format_date().xpect_eq("2026-09-23");
/// ```
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct Tid(u64);

impl Tid {
	/// The length of the written form.
	pub const LEN: usize = 13;
	/// The base32-sortable alphabet: ascending byte order is ascending value.
	const ALPHABET: &'static [u8; 32] = b"234567abcdefghijklmnopqrstuvwxyz";
	/// The bits the clock id takes.
	const CLOCK_BITS: u32 = 10;
	const CLOCK_MASK: u64 = (1 << Self::CLOCK_BITS) - 1;

	/// Mint a fresh TID from the wall clock, monotonic within this process.
	#[cfg(feature = "rand")]
	pub fn now() -> Self {
		/// `(clock id, last minted microseconds)`, the clock id chosen once.
		static MINTER: RwLock<Option<(u16, u64)>> = RwLock::new(None);
		let now = time_ext::now().as_micros() as u64;
		let mut minter = MINTER
			.write()
			.unwrap_or_else(bevy::platform::sync::PoisonError::into_inner);
		let (clock_id, last) = *minter.get_or_insert_with(|| {
			(
				RandomSource::default().random::<u16>()
					& Self::CLOCK_MASK as u16,
				0,
			)
		});
		let micros = now.max(last + 1);
		*minter = Some((clock_id, micros));
		Self::from_parts(micros, clock_id)
	}

	/// The TID of `micros` since the epoch under `clock_id`, its top bits
	/// discarded so the result keeps the zero high bit.
	pub fn from_parts(micros: u64, clock_id: u16) -> Self {
		Self(
			((micros << Self::CLOCK_BITS)
				| (clock_id as u64 & Self::CLOCK_MASK))
				& (u64::MAX >> 1),
		)
	}

	/// Microseconds since the Unix epoch.
	pub fn micros(&self) -> u64 { self.0 >> Self::CLOCK_BITS }

	/// The minting process's clock id.
	pub fn clock_id(&self) -> u16 { (self.0 & Self::CLOCK_MASK) as u16 }

	/// The instant this TID was minted at, to the millisecond.
	pub fn timestamp(&self) -> Timestamp {
		Timestamp::from_millis((self.micros() / 1000) as i64)
	}

	/// Parse the 13 character written form.
	pub fn parse(text: &str) -> Result<Self> {
		if text.len() != Self::LEN {
			bevybail!(
				"tid `{text}` must be {} characters, not {}",
				Self::LEN,
				text.len()
			);
		}
		let mut value = 0u64;
		for (i, byte) in text.bytes().enumerate() {
			let Some(digit) =
				Self::ALPHABET.iter().position(|item| *item == byte)
			else {
				bevybail!(
					"tid `{text}` contains `{}`: only `2-7a-z` are allowed",
					byte as char
				);
			};
			// the first character carries the zero high bit, so its top
			// half of the alphabet is out of range
			if i == 0 && digit >= 16 {
				bevybail!("tid `{text}` sets the reserved high bit");
			}
			value = (value << 5) | digit as u64;
		}
		Self(value).xok()
	}
}

impl From<Timestamp> for Tid {
	fn from(timestamp: Timestamp) -> Self {
		Self::from_parts(timestamp.millis().max(0) as u64 * 1000, 0)
	}
}

impl core::fmt::Display for Tid {
	fn fmt(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
		let mut out = [0u8; Self::LEN];
		let mut value = self.0;
		for slot in out.iter_mut().rev() {
			*slot = Self::ALPHABET[(value & 31) as usize];
			value >>= 5;
		}
		// the alphabet is ascii, so every slot is a whole char
		formatter.write_str(core::str::from_utf8(&out).unwrap_or_default())
	}
}

impl core::str::FromStr for Tid {
	type Err = BevyError;
	fn from_str(text: &str) -> Result<Self> { Self::parse(text) }
}

impl TryFrom<SmolStr> for Tid {
	type Error = BevyError;
	fn try_from(text: SmolStr) -> Result<Self> { Self::parse(&text) }
}

impl From<Tid> for SmolStr {
	fn from(tid: Tid) -> SmolStr { SmolStr::new(tid.to_string()) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn round_trips() {
		let tid = Tid::from_parts(1_759_000_000_123_456, 513);
		tid.micros().xpect_eq(1_759_000_000_123_456);
		tid.clock_id().xpect_eq(513);
		tid.to_string().len().xpect_eq(Tid::LEN);
		Tid::parse(&tid.to_string()).unwrap().xpect_eq(tid);
	}

	/// The string order is the numeric order, which is why a collection
	/// listed by rkey lists by creation time.
	#[crate::test]
	fn orders_as_its_string() {
		let earlier = Tid::from_parts(1_000, 7);
		let later = Tid::from_parts(1_001, 0);
		(earlier < later).xpect_true();
		(earlier.to_string() < later.to_string()).xpect_true();
	}

	#[crate::test]
	fn from_a_timestamp_zeroes_the_clock() {
		let tid = Tid::from(Timestamp::from_millis(1_759_000_000_123));
		tid.clock_id().xpect_eq(0);
		tid.timestamp().millis().xpect_eq(1_759_000_000_123);
	}

	#[crate::test]
	fn rejects_malformed() {
		Tid::parse("3mw72aaeuj22").unwrap_err();
		Tid::parse("3mw72aaeuj22N").unwrap_err();
		// `z` first sets the reserved high bit
		Tid::parse("zzzzzzzzzzzzz")
			.unwrap_err()
			.to_string()
			.xpect_contains("high bit");
	}

	/// Two mints in one microsecond still differ: the minter steps past its
	/// last value rather than repeating it.
	#[cfg(feature = "rand")]
	#[crate::test]
	fn mints_are_monotonic() {
		let mints = (0..1000).map(|_| Tid::now()).collect::<Vec<_>>();
		for pair in mints.windows(2) {
			(pair[0] < pair[1]).xpect_true();
		}
		mints[0].clock_id().xpect_eq(mints[999].clock_id());
	}
}
