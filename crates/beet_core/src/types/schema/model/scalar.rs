//! Scalar schema and constraint declarations used by [`ValueSchema`].
use crate::prelude::*;

/// Generates a numeric schema and its constraint declarations.
macro_rules! number_schema {
	(
		$schema:ident,
		$constraint:ident,
		$min:ident,
		$max:ident,
		$step:ident,
		$t:ty
	) => {
		#[doc = concat!("Constraint applied to a [`", stringify!($schema), "`].")]
		#[derive(
			Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
		)]
		#[cfg_attr(
			feature = "serde",
			derive(serde::Serialize, serde::Deserialize)
		)]
		pub enum $constraint {
			/// The value must be at least this number.
			Min($min),
			/// The value must be at most this number.
			Max($max),
			/// The value must be a multiple of this step from zero.
			Step($step),
		}

		#[doc = concat!("Minimum-value constraint for [`", stringify!($schema), "`].")]
		#[derive(
			Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
		)]
		#[cfg_attr(
			feature = "serde",
			derive(serde::Serialize, serde::Deserialize)
		)]
		pub struct $min {
			/// The minimum allowed value.
			pub value: $t,
			/// What to do if `value` falls below the minimum.
			pub behavior: ConstraintBehavior,
		}

		#[doc = concat!("Maximum-value constraint for [`", stringify!($schema), "`].")]
		#[derive(
			Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
		)]
		#[cfg_attr(
			feature = "serde",
			derive(serde::Serialize, serde::Deserialize)
		)]
		pub struct $max {
			/// The maximum allowed value.
			pub value: $t,
			/// What to do if `value` exceeds the maximum.
			pub behavior: ConstraintBehavior,
		}

		#[doc = concat!("Step (modulus) constraint for [`", stringify!($schema), "`].")]
		#[derive(
			Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
		)]
		#[cfg_attr(
			feature = "serde",
			derive(serde::Serialize, serde::Deserialize)
		)]
		pub struct $step {
			/// The step value.
			pub value: $t,
			/// What to do if `value` is not aligned to the step.
			pub behavior: ConstraintBehavior,
		}

		#[doc = concat!("Schema for a [`", stringify!($t), "`] value.")]
		#[derive(
			Debug,
			Default,
			Clone,
			PartialEq,
			Eq,
			PartialOrd,
			Ord,
			Hash,
			Reflect,
		)]
		#[cfg_attr(
			feature = "serde",
			derive(serde::Serialize, serde::Deserialize)
		)]
		pub struct $schema {
			/// Constraints applied to this number.
			pub constraints: Vec<$constraint>,
		}
	};
}

number_schema!(I64Schema, I64Constraint, I64Min, I64Max, I64Step, i64);
number_schema!(U64Schema, U64Constraint, U64Min, U64Max, U64Step, u64);

/// Constraint applied to an [`F64Schema`].
#[derive(Debug, Clone, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum F64Constraint {
	/// The value must be at least this number.
	Min(F64Min),
	/// The value must be at most this number.
	Max(F64Max),
	/// The value must be a multiple of this step from zero.
	Step(F64Step),
}

/// Minimum-value constraint for [`F64Schema`].
#[derive(Debug, Clone, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct F64Min {
	/// The minimum allowed value.
	pub value: f64,
	/// What to do if `value` falls below the minimum.
	pub behavior: ConstraintBehavior,
}
/// Maximum-value constraint for [`F64Schema`].
#[derive(Debug, Clone, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct F64Max {
	/// The maximum allowed value.
	pub value: f64,
	/// What to do if `value` exceeds the maximum.
	pub behavior: ConstraintBehavior,
}
/// Step (modulus) constraint for [`F64Schema`].
#[derive(Debug, Clone, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct F64Step {
	/// The step value.
	pub value: f64,
	/// What to do if `value` is not aligned to the step.
	pub behavior: ConstraintBehavior,
}

/// Schema for an [`f64`] value.
#[derive(Debug, Default, Clone, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct F64Schema {
	/// Constraints applied to this number.
	pub constraints: Vec<F64Constraint>,
}

// f64 doesn't impl Eq/Ord/Hash. Implement them by bit pattern so [`F64Schema`]
// can be embedded in larger Hash/Ord-deriving structures.
impl Eq for F64Min {}
impl Eq for F64Max {}
impl Eq for F64Step {}
impl Eq for F64Constraint {}
impl Eq for F64Schema {}

macro_rules! float_impls {
	($name:ident) => {
		impl core::hash::Hash for $name {
			fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
				self.value.to_bits().hash(state);
				self.behavior.hash(state);
			}
		}
		impl Ord for $name {
			fn cmp(&self, other: &Self) -> core::cmp::Ordering {
				self.value
					.to_bits()
					.cmp(&other.value.to_bits())
					.then(self.behavior.cmp(&other.behavior))
			}
		}
		impl PartialOrd for $name {
			fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
				Some(self.cmp(other))
			}
		}
	};
}
float_impls!(F64Min);
float_impls!(F64Max);
float_impls!(F64Step);

impl core::hash::Hash for F64Constraint {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		core::mem::discriminant(self).hash(state);
		match self {
			Self::Min(constraint) => constraint.hash(state),
			Self::Max(constraint) => constraint.hash(state),
			Self::Step(constraint) => constraint.hash(state),
		}
	}
}
impl Ord for F64Constraint {
	fn cmp(&self, other: &Self) -> core::cmp::Ordering {
		use core::cmp::Ordering;
		match (self, other) {
			(Self::Min(left), Self::Min(right)) => left.cmp(right),
			(Self::Max(left), Self::Max(right)) => left.cmp(right),
			(Self::Step(left), Self::Step(right)) => left.cmp(right),
			(Self::Min(_), _) => Ordering::Less,
			(_, Self::Min(_)) => Ordering::Greater,
			(Self::Max(_), _) => Ordering::Less,
			(_, Self::Max(_)) => Ordering::Greater,
		}
	}
}
impl PartialOrd for F64Constraint {
	fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
		Some(self.cmp(other))
	}
}
impl core::hash::Hash for F64Schema {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		self.constraints.hash(state);
	}
}
impl Ord for F64Schema {
	fn cmp(&self, other: &Self) -> core::cmp::Ordering {
		self.constraints.cmp(&other.constraints)
	}
}
impl PartialOrd for F64Schema {
	fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

/// Constraint applied to a string value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StringConstraint {
	/// Minimum length in chars.
	MinLength {
		/// The minimum allowed length.
		value: usize,
		/// What to do if the string is too short.
		behavior: ConstraintBehavior,
	},
	/// Maximum length in chars.
	MaxLength {
		/// The maximum allowed length.
		value: usize,
		/// What to do if the string is too long.
		behavior: ConstraintBehavior,
	},
	/// String must look like an email address.
	Email,
}

/// Schema for a string value.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StringSchema {
	/// Whether this value is sensitive (password etc), should be hidden
	/// from logs and rendered as `***`.
	pub sensitive: bool,
	/// Whether this value is prose spanning several lines, ie a `<textarea>`
	/// rather than a single-line input.
	pub multiline: bool,
	/// Additional constraints.
	pub constraints: Vec<StringConstraint>,
}

impl StringSchema {
	/// Mark this string as sensitive.
	pub fn sensitive(mut self) -> Self {
		self.sensitive = true;
		self
	}
	/// Mark this string as multiline prose.
	pub fn multiline(mut self) -> Self {
		self.multiline = true;
		self
	}
	/// Add a constraint.
	pub fn with(mut self, constraint: StringConstraint) -> Self {
		self.constraints.push(constraint);
		self
	}
}

/// Schema for a boolean value.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoolSchema {}

/// Schema for an [`Entity`] reference.
///
/// A reference is a node key, not a number: it serializes through the same
/// entity map the surrounding document's node keys do, so it survives a save
/// and reload, and a UI dispatching on this kind renders a node picker rather
/// than a number input.
///
/// On the wire it is the bits of the generation-stripped file entity, which is
/// how bevy writes an `Entity`, so [`node_key`](Self::node_key) and
/// [`reference`](Self::reference) are the one place a document reads and
/// writes the key a reference names.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntitySchema {}

impl EntitySchema {
	/// The node key an entity reference names, `None` when `value` is not a
	/// reference.
	pub fn node_key(value: &Value) -> Option<u32> {
		let bits = match value {
			Value::Uint(bits) => *bits,
			Value::Int(bits) => u64::try_from(*bits).ok()?,
			_ => return None,
		};
		Entity::try_from_bits(bits).map(|entity| entity.index_u32())
	}

	/// An entity reference naming the node at `key`, as the surrounding
	/// document writes one.
	pub fn reference(key: u32) -> Result<Value> {
		Entity::from_raw_u32(key)
			.ok_or_else(|| bevyhow!("`{key}` is not a valid node key"))?
			.to_bits()
			.xmap(Value::Uint)
			.xok()
	}
}

/// Schema for a bytes value.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BytesSchema {
	/// Optional max byte length.
	pub max_len: Option<usize>,
}
