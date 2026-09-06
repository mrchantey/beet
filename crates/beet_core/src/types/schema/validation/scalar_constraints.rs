//! Private scalar constraint execution used by schema validation.
use crate::prelude::*;

/// A future returned by [`ApplyConstraints::apply`].
///
/// Carries the borrows for `self`, the path and the value, so apply impls can
/// freely borrow without `'static`.
pub(super) type ApplyFuture<'a> =
	core::pin::Pin<Box<dyn 'a + Send + Future<Output = Vec<ValidationError>>>>;

/// Applies a scalar schema's constraints, possibly mutating the value.
pub(super) trait ApplyConstraints {
	/// The scalar value type.
	type Value;

	/// Apply every constraint at `path`.
	fn apply<'a>(
		&'a self,
		path: &'a FieldPath,
		value: &'a mut Self::Value,
	) -> ApplyFuture<'a>;
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
									let mut string = value.to_string();
									while string.chars().count() < *min {
										string.push(' ');
									}
									*value = SmolStr::from(string);
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
						let string = value.as_str();
						let valid = string
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

impl ApplyConstraints for EntitySchema {
	type Value = u64;

	fn apply<'a>(
		&'a self,
		_path: &'a FieldPath,
		_value: &'a mut Self::Value,
	) -> ApplyFuture<'a> {
		Box::pin(async { Vec::new() })
	}
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

macro_rules! impl_number_constraints {
	($schema:ident, $constraint:ident, $value:ty, $apply_step:ident) => {
		impl ApplyConstraints for $schema {
			type Value = $value;

			fn apply<'a>(
				&'a self,
				path: &'a FieldPath,
				value: &'a mut Self::Value,
			) -> ApplyFuture<'a> {
				Box::pin(async move {
					let mut errors = Vec::new();
					for constraint in &self.constraints {
						match constraint {
							$constraint::Min(constraint) => apply_min(
								constraint.value,
								constraint.behavior,
								path,
								value,
								&mut errors,
							),
							$constraint::Max(constraint) => apply_max(
								constraint.value,
								constraint.behavior,
								path,
								value,
								&mut errors,
							),
							$constraint::Step(constraint) => $apply_step(
								constraint.value,
								constraint.behavior,
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
	};
}

impl_number_constraints!(I64Schema, I64Constraint, i64, apply_step_int);
impl_number_constraints!(U64Schema, U64Constraint, u64, apply_step_int);
impl_number_constraints!(F64Schema, F64Constraint, f64, apply_step_float);

fn apply_min<T>(
	min: T,
	behavior: ConstraintBehavior,
	path: &FieldPath,
	value: &mut T,
	errors: &mut Vec<ValidationError>,
) where
	T: Copy + PartialOrd + core::fmt::Display,
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
	T: Copy + PartialOrd + core::fmt::Display,
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
		+ PartialEq
		+ core::fmt::Display
		+ core::ops::Rem<Output = T>
		+ core::ops::Sub<Output = T>,
{
	let remainder = *value % step;
	if remainder != T::default() {
		match behavior {
			ConstraintBehavior::Error => {
				errors.push(ValidationError::new(
					path.clone(),
					format!("must be a multiple of {}", step),
				));
			}
			ConstraintBehavior::Mutate => *value = *value - remainder,
		}
	}
}

fn apply_step_float(
	step: f64,
	behavior: ConstraintBehavior,
	path: &FieldPath,
	value: &mut f64,
	errors: &mut Vec<ValidationError>,
) {
	let remainder = *value % step;
	if remainder != 0.0 {
		match behavior {
			ConstraintBehavior::Error => {
				errors.push(ValidationError::new(
					path.clone(),
					format!("must be a multiple of {}", step),
				));
			}
			ConstraintBehavior::Mutate => *value -= remainder,
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	async fn validate_min_constraint() {
		let schema = ValueSchema::I64(I64Schema {
			constraints: vec![I64Constraint::Min(I64Min {
				value: 10,
				behavior: ConstraintBehavior::Error,
			})],
		});
		let mut value = value!(5);
		let errors = schema.validate(&mut value).await;
		errors.len().xpect_eq(1);
		// no mutation
		value.as_i64().unwrap().xpect_eq(5);
	}

	#[crate::test]
	async fn validate_min_mutate() {
		let schema = ValueSchema::I64(I64Schema {
			constraints: vec![I64Constraint::Min(I64Min {
				value: 10,
				behavior: ConstraintBehavior::Mutate,
			})],
		});
		let mut value = value!(5);
		let errors = schema.validate(&mut value).await;
		errors.is_empty().xpect_true();
		value.as_i64().unwrap().xpect_eq(10);
	}

	#[crate::test]
	async fn validate_string_min_length() {
		let schema = ValueSchema::String(StringSchema::default().with(
			StringConstraint::MinLength {
				value: 3,
				behavior: ConstraintBehavior::Error,
			},
		));
		let mut value = value!("hi");
		let errors = schema.validate(&mut value).await;
		errors.len().xpect_eq(1);
	}
}
