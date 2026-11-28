use crate::prelude::*;

/// What a write requires of the version already at its key, atproto's
/// `swapRecord` semantics: checked and applied as one step, so two writers
/// never silently erase each other. One type for every keyed write that hashes
/// what it holds, a table row and a PDS record alike.
///
/// ```
/// # use beet_core::prelude::*;
/// let cid = Cid::raw(b"hello");
/// Precondition::Any.holds(None).xpect_true();
/// Precondition::Absent.holds(Some(&cid)).xpect_false();
/// Precondition::Matches(cid.clone()).holds(Some(&cid)).xpect_true();
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Precondition {
	/// Whatever is there, including nothing.
	#[default]
	Any,
	/// Nothing is there: a create.
	Absent,
	/// The version there has exactly this cid: a swap.
	Matches(Cid),
}

impl Precondition {
	/// Whether this holds of the version whose cid is `found`, `None` for
	/// nothing there.
	pub fn holds(&self, found: Option<&Cid>) -> bool {
		match (self, found) {
			(Self::Any, _) => true,
			(Self::Absent, found) => found.is_none(),
			(Self::Matches(expected), Some(found)) => expected == found,
			(Self::Matches(_), None) => false,
		}
	}

	/// How an error names `found`, the cid there or `nothing`.
	pub fn describe_found(found: Option<&Cid>) -> String {
		found.map_or_else(|| "nothing".to_string(), ToString::to_string)
	}
}

impl core::fmt::Display for Precondition {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Any => write!(f, "anything"),
			Self::Absent => write!(f, "nothing"),
			Self::Matches(cid) => write!(f, "{cid}"),
		}
	}
}
