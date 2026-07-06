use crate::prelude::Duration;
use extend::ext;

/// Serde for a [`Duration`] as a unit-suffixed string, eg `"1.5s"` or `"800ms"` — the
/// format [`Duration::from_human_str`] parses and a [`Duration`] markup attribute coerces
/// from. Use via `#[serde(with = "beet_core::prelude::duration_str")]`.
///
/// Serialization always writes a seconds string (`"1.5s"`). Deserialization REQUIRES a
/// unit: a bare number (a JSON number `1.5`, or a unit-less string `"1.5"`) errors rather
/// than silently assuming seconds. Reflect renders a [`Duration`] as this same string, so a
/// model authoring a value against the reflected tool schema emits one the deserializer
/// accepts. Upstreamed here so any crate serializing a wire/config [`Duration`] adopts the
/// same unit-required shape rather than re-deriving a lenient one.
#[cfg(feature = "serde")]
pub mod duration_str {
	use crate::prelude::*;
	use core::fmt;
	use serde::Deserializer;
	use serde::Serializer;
	use serde::de::Visitor;

	/// Serialize as a unit-suffixed seconds string, eg `"1.5s"`.
	pub fn serialize<S>(
		duration: &Duration,
		serializer: S,
	) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&format!("{}s", duration.as_secs_f64()))
	}

	/// Deserialize a unit-suffixed string like `"1.5s"`. A bare number — a JSON number
	/// or a unit-less string like `"1.5"` — errors, so a unit is always required.
	pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_str(DurationVisitor)
	}

	struct DurationVisitor;

	impl Visitor<'_> for DurationVisitor {
		type Value = Duration;

		fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
			f.write_str("a unit-suffixed duration like \"1.5s\" or \"800ms\"")
		}

		fn visit_str<E>(self, value: &str) -> Result<Duration, E>
		where
			E: serde::de::Error,
		{
			Duration::from_human_str(value).ok_or_else(|| {
				E::custom(format!(
					"expected a unit-suffixed duration like \"1.5s\" or \"800ms\", got {value:?}"
				))
			})
		}
	}
}

/// Extension trait for [`Duration`] providing convenient constructors.
#[ext]
pub impl Duration {
	/// Creates a duration from hours, minutes, and seconds.
	///
	/// # Examples
	///
	/// ```
	/// use beet_core::prelude::*;
	///
	/// let duration = Duration::from_hms(1, 30, 45);
	/// assert_eq!(duration.as_secs(), 5445); // 1*3600 + 30*60 + 45
	/// ```
	fn from_hms(hour: u64, minute: u64, second: u64) -> Duration {
		Duration::from_secs(hour * 3600 + minute * 60 + second)
	}

	/// Parse a human duration like `"50ms"`, `"1.5s"`, `"2m"`, `"1h"` or `"7d"`. The
	/// unit (`ns`, `us`/`µs`, `ms`, `s`, `m`, `h`, `d`) is required; returns `None` on a
	/// missing or unknown unit, so a bare number is never silently assumed to be seconds.
	/// Shared by markup attribute coercion and the drive-command wire serde.
	fn from_human_str(string: &str) -> Option<Duration> {
		let string = string.trim();
		let split = string
			.find(|c: char| !c.is_ascii_digit() && c != '.')
			.unwrap_or(string.len());
		let (number, unit) = string.split_at(split);
		let number: f64 = number.parse().ok()?;
		let secs = match unit.trim() {
			"ns" => number / 1_000_000_000.0,
			"us" | "µs" => number / 1_000_000.0,
			"ms" => number / 1_000.0,
			"s" => number,
			"m" => number * 60.0,
			"h" => number * 60.0 * 60.0,
			"d" => number * 24.0 * 60.0 * 60.0,
			_ => return None,
		};
		Some(Duration::from_secs_f64(secs))
	}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;

	/// A single [`Duration`] field, serialized via the [`duration_str`] helper.
	#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
	struct Wrap(#[serde(with = "crate::prelude::duration_str")] Duration);

	/// [`duration_str`] serializes a unit-suffixed string and round-trips one back;
	/// a bare number — a JSON number or a unit-less string — is rejected, so a unit
	/// is always required.
	#[beet_core::test]
	fn duration_str_requires_a_unit() {
		// a unit string round-trips as `"1.5s"`
		let json =
			serde_json::to_string(&Wrap(Duration::from_secs_f64(1.5))).unwrap();
		json.xpect_eq(r#""1.5s""#.to_string());
		serde_json::from_str::<Wrap>(&json)
			.unwrap()
			.xpect_eq(Wrap(Duration::from_secs_f64(1.5)));
		// other units decode
		serde_json::from_str::<Wrap>(r#""250ms""#)
			.unwrap()
			.0
			.xpect_eq(Duration::from_millis(250));
		// a bare number is rejected, whether a JSON number or a unit-less string
		serde_json::from_str::<Wrap>("1.5").is_err().xpect_true();
		serde_json::from_str::<Wrap>(r#""1.5""#).is_err().xpect_true();
	}
}
