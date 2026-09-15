//! A retention window scoped to one key prefix of a bucket, whichever provider
//! holds it.

use beet_core::prelude::*;

/// An expiry scoped to one key prefix of a bucket, ie the nightly SQLite
/// snapshots under `sqlite/` in an archive whose other prefixes hold the only
/// copy of what is in them and expire never.
///
/// Authored inline on the bucket, ie
/// `expire_prefixes={[{prefix:"sqlite/", expire_days:180}]}`. The prefix is a
/// writer convention rather than a boundary (a grant is whole-bucket), so this
/// is where a bucket says which of its conventions are disposable. Provider
/// agnostic: an `S3BucketBlock` renders it as a lifecycle rule and an
/// `R2BucketBlock` as an age transition, each in its own shape, from the one
/// declaration.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	Get,
	SetWith,
	Serialize,
	Deserialize,
	Reflect,
)]
#[reflect(Default)]
pub struct PrefixExpiry {
	/// The literal key prefix the rule filters on, ie `sqlite/`. The trailing
	/// slash is load-bearing: `sqlite` also matches `sqlite-old/`, so a prefix
	/// naming a directory should say so.
	prefix: SmolStr,
	/// Days an object under [`prefix`](Self::prefix) is kept. Must be positive,
	/// since a rule expiring in zero days says nothing and every provider
	/// rejects it.
	expire_days: i64,
}

impl PrefixExpiry {
	pub fn new(prefix: impl Into<SmolStr>, expire_days: i64) -> Self {
		Self {
			prefix: prefix.into(),
			expire_days,
		}
	}

	/// The rule id, ie `expire-sqlite`. A function of the prefix rather than
	/// of the declaration order, so reordering declarations never diffs a
	/// rendered configuration.
	pub fn rule_id(&self) -> String {
		self.prefix
			.chars()
			.map(|char| match char.is_ascii_alphanumeric() {
				true => char,
				false => '-',
			})
			.collect::<String>()
			.trim_matches('-')
			.xmap(|prefix| format!("expire-{prefix}"))
	}

	/// Rejects a declaration no provider would accept, at render rather than
	/// at apply.
	pub fn validate(&self) -> Result {
		if self.prefix.is_empty() {
			bevybail!(
				"a prefix expiry declares no prefix; the whole bucket is `expire_days`"
			);
		}
		if self.expire_days <= 0 {
			bevybail!(
				"prefix expiry '{}' declares {} days; a prefix that expires nothing is simply not declared",
				self.prefix,
				self.expire_days
			);
		}
		Ok(())
	}

	/// Fail the render when two rules would share an id, which every provider
	/// rejects and which two prefixes sanitizing to the same id (`logs/` and
	/// `logs-`) would otherwise produce at apply time.
	pub fn assert_unique_ids<'a>(
		label: &str,
		ids: impl IntoIterator<Item = &'a str>,
	) -> Result {
		let mut seen = HashSet::<&str>::default();
		for id in ids {
			if !seen.insert(id) {
				bevybail!(
					"bucket '{label}' declares two lifecycle rules with id '{id}'"
				);
			}
		}
		Ok(())
	}
}
