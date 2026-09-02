//! Per-variant schema types and constraints used by [`ValueSchema`].
//!
//! Each schema type owns its constraint list and implements [`ApplyConstraints`]
//! so it can be invoked uniformly during validation.
use super::*;
use crate::prelude::*;

// ── Numeric constraints ────────────────────────────────────────────────

/// Generates a numeric schema and its constraint enum for a single primitive.
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
			Self::Min(c) => c.hash(state),
			Self::Max(c) => c.hash(state),
			Self::Step(c) => c.hash(state),
		}
	}
}
impl Ord for F64Constraint {
	fn cmp(&self, other: &Self) -> core::cmp::Ordering {
		use core::cmp::Ordering;
		match (self, other) {
			(Self::Min(a), Self::Min(b)) => a.cmp(b),
			(Self::Max(a), Self::Max(b)) => a.cmp(b),
			(Self::Step(a), Self::Step(b)) => a.cmp(b),
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

// ── String constraints ─────────────────────────────────────────────────

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
	/// Additional constraints.
	pub constraints: Vec<StringConstraint>,
}

impl StringSchema {
	/// Mark this string as sensitive.
	pub fn sensitive(mut self) -> Self {
		self.sensitive = true;
		self
	}
	/// Add a constraint.
	pub fn with(mut self, constraint: StringConstraint) -> Self {
		self.constraints.push(constraint);
		self
	}
}

impl ApplyConstraints for StringSchema {
	type Value = SmolStr;
	fn apply<'a>(
		&'a self,
		path: &'a FieldPath,
		value: &'a mut Self::Value,
	) -> ApplyFuture<'a> {
		Box::pin(async move {
			let mut errors = Vec::new();
			for constraint in &self.constraints {
				match constraint {
					StringConstraint::MinLength {
						value: min,
						behavior,
					} => {
						let len = value.chars().count();
						if len < *min {
							match behavior {
								ConstraintBehavior::Error => {
									errors.push(ValidationError::new(
										path.clone(),
										format!(
											"must be at least {} characters",
											min
										),
									));
								}
								ConstraintBehavior::Mutate => {
									let mut s = value.to_string();
									while s.chars().count() < *min {
										s.push(' ');
									}
									*value = SmolStr::from(s);
								}
							}
						}
					}
					StringConstraint::MaxLength {
						value: max,
						behavior,
					} => {
						let len = value.chars().count();
						if len > *max {
							match behavior {
								ConstraintBehavior::Error => {
									errors.push(ValidationError::new(
										path.clone(),
										format!(
											"must be at most {} characters",
											max
										),
									));
								}
								ConstraintBehavior::Mutate => {
									let truncated: String =
										value.chars().take(*max).collect();
									*value = SmolStr::from(truncated);
								}
							}
						}
					}
					StringConstraint::Email => {
						// Minimal email check: contains '@' and a '.' after it.
						let s = value.as_str();
						let valid = s
							.split_once('@')
							.is_some_and(|(_, rhs)| rhs.contains('.'));
						if !valid {
							errors.push(ValidationError::new(
								path.clone(),
								"must be a valid email address",
							));
						}
					}
				}
			}
			errors
		})
	}
}

/// Schema for a boolean value.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoolSchema {}

impl ApplyConstraints for BoolSchema {
	type Value = bool;
	fn apply<'a>(
		&'a self,
		_path: &'a FieldPath,
		_value: &'a mut Self::Value,
	) -> ApplyFuture<'a> {
		Box::pin(async { Vec::new() })
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

impl ApplyConstraints for BytesSchema {
	type Value = Vec<u8>;
	fn apply<'a>(
		&'a self,
		path: &'a FieldPath,
		value: &'a mut Self::Value,
	) -> ApplyFuture<'a> {
		Box::pin(async move {
			let mut errors = Vec::new();
			if let Some(max) = self.max_len
				&& value.len() > max
			{
				errors.push(ValidationError::new(
					path.clone(),
					format!("must be at most {} bytes", max),
				));
			}
			errors
		})
	}
}

// ── Numeric apply impls ────────────────────────────────────────────────

impl ApplyConstraints for I64Schema {
	type Value = i64;
	fn apply<'a>(
		&'a self,
		path: &'a FieldPath,
		value: &'a mut Self::Value,
	) -> ApplyFuture<'a> {
		Box::pin(async move {
			let mut errors = Vec::new();
			for c in &self.constraints {
				match c {
					I64Constraint::Min(c) => {
						apply_min(c.value, c.behavior, path, value, &mut errors)
					}
					I64Constraint::Max(c) => {
						apply_max(c.value, c.behavior, path, value, &mut errors)
					}
					I64Constraint::Step(c) => apply_step_int(
						c.value,
						c.behavior,
						path,
						value,
						&mut errors,
					),
				}
			}
			errors
		})
	}
}
impl ApplyConstraints for U64Schema {
	type Value = u64;
	fn apply<'a>(
		&'a self,
		path: &'a FieldPath,
		value: &'a mut Self::Value,
	) -> ApplyFuture<'a> {
		Box::pin(async move {
			let mut errors = Vec::new();
			for c in &self.constraints {
				match c {
					U64Constraint::Min(c) => {
						apply_min(c.value, c.behavior, path, value, &mut errors)
					}
					U64Constraint::Max(c) => {
						apply_max(c.value, c.behavior, path, value, &mut errors)
					}
					U64Constraint::Step(c) => apply_step_int(
						c.value,
						c.behavior,
						path,
						value,
						&mut errors,
					),
				}
			}
			errors
		})
	}
}
impl ApplyConstraints for F64Schema {
	type Value = f64;
	fn apply<'a>(
		&'a self,
		path: &'a FieldPath,
		value: &'a mut Self::Value,
	) -> ApplyFuture<'a> {
		Box::pin(async move {
			let mut errors = Vec::new();
			for c in &self.constraints {
				match c {
					F64Constraint::Min(c) => {
						apply_min(c.value, c.behavior, path, value, &mut errors)
					}
					F64Constraint::Max(c) => {
						apply_max(c.value, c.behavior, path, value, &mut errors)
					}
					F64Constraint::Step(c) => {
						let rem = *value % c.value;
						if rem != 0.0 {
							match c.behavior {
								ConstraintBehavior::Error => {
									errors.push(ValidationError::new(
										path.clone(),
										format!(
											"must be a multiple of {}",
											c.value
										),
									));
								}
								ConstraintBehavior::Mutate => *value -= rem,
							}
						}
					}
				}
			}
			errors
		})
	}
}

fn apply_min<T>(
	min: T,
	behavior: ConstraintBehavior,
	path: &FieldPath,
	value: &mut T,
	errors: &mut Vec<ValidationError>,
) where
	T: PartialOrd + Copy + core::fmt::Display,
{
	if *value < min {
		match behavior {
			ConstraintBehavior::Error => {
				errors.push(ValidationError::new(
					path.clone(),
					format!("must be at least {}", min),
				));
			}
			ConstraintBehavior::Mutate => *value = min,
		}
	}
}
fn apply_max<T>(
	max: T,
	behavior: ConstraintBehavior,
	path: &FieldPath,
	value: &mut T,
	errors: &mut Vec<ValidationError>,
) where
	T: PartialOrd + Copy + core::fmt::Display,
{
	if *value > max {
		match behavior {
			ConstraintBehavior::Error => {
				errors.push(ValidationError::new(
					path.clone(),
					format!("must be at most {}", max),
				));
			}
			ConstraintBehavior::Mutate => *value = max,
		}
	}
}
fn apply_step_int<T>(
	step: T,
	behavior: ConstraintBehavior,
	path: &FieldPath,
	value: &mut T,
	errors: &mut Vec<ValidationError>,
) where
	T: Default
		+ Copy
		+ core::fmt::Display
		+ PartialEq
		+ core::ops::Rem<Output = T>
		+ core::ops::Sub<Output = T>,
{
	let rem = *value % step;
	if rem != T::default() {
		match behavior {
			ConstraintBehavior::Error => {
				errors.push(ValidationError::new(
					path.clone(),
					format!("must be a multiple of {}", step),
				));
			}
			ConstraintBehavior::Mutate => *value = *value - rem,
		}
	}
}

// ── Composite schemas ──────────────────────────────────────────────────

/// A field within a [`StructSchema`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NamedFieldSchema {
	/// The map key for this field.
	pub key: SmolStr,
	/// Whether this field must be present.
	pub required: bool,
	/// Optional human readable label, falls back to `key` if missing.
	pub label: Option<SmolStr>,
	/// Optional description.
	pub description: Option<SmolStr>,
	/// The field's value schema.
	pub schema: ValueSchema,
}

/// A field within a [`TupleSchema`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UnnamedFieldSchema {
	/// Whether this field must be present.
	pub required: bool,
	/// Optional description.
	pub description: Option<SmolStr>,
	/// The field's value schema.
	pub schema: ValueSchema,
}

/// Schema for a struct-shaped value.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StructSchema {
	/// The type's short name, if known.
	pub name: Option<SmolStr>,
	/// Whether keys not in [`fields`](Self::fields) are permitted.
	pub allow_additional: bool,
	/// Field schemas.
	pub fields: Vec<NamedFieldSchema>,
}

/// Schema for a fixed-arity tuple value, also used for tuple structs.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TupleSchema {
	/// The type's short name, if known.
	pub name: Option<SmolStr>,
	/// Field schemas in order.
	pub fields: Vec<UnnamedFieldSchema>,
}

/// Schema for a homogenous list value, also used for arrays and sets.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[reflect(opaque)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ListSchema {
	/// The schema each element must satisfy.
	pub item: Box<ValueSchema>,
	/// Minimum number of elements.
	pub min_items: Option<usize>,
	/// Maximum number of elements.
	pub max_items: Option<usize>,
	/// Whether duplicate elements are forbidden.
	pub unique: bool,
}

/// Schema for a map value with string keys.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[reflect(opaque)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MapSchema {
	/// The schema each value in the map must satisfy.
	pub value: Box<ValueSchema>,
}

/// A variant within an [`EnumSchema`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VariantSchema {
	/// The variant's name as it appears in serialized form.
	pub name: SmolStr,
	/// Optional payload schema; `None` for unit variants.
	pub payload: Option<ValueSchema>,
}

/// Schema for an enum value, externally tagged: `{"VariantName": payload}` or
/// the bare string `"VariantName"` for unit variants.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EnumSchema {
	/// The type's short name, if known.
	pub name: Option<SmolStr>,
	/// Variants in declaration order.
	pub variants: Vec<VariantSchema>,
}
