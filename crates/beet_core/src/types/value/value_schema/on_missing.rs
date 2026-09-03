//! [`OnMissing`]: the one per-field policy for an absent value.
use crate::prelude::*;

/// What to do when a field has no value.
///
/// One policy shared by the two layers that need it: a [`NamedFieldSchema`]
/// declares it for a schema field, and [`FieldRef`](crate::prelude::FieldRef)
/// declares it for a bound widget.
///
/// A schema field's policy is consulted **only** by the commit path
/// ([`SchemaCommit`]), never at read time: data must never observably violate
/// its schema, so a required field missing at read is an error naming the
/// field, not a silently substituted default.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OnMissing {
	/// Fail, naming the field.
	Error,
	/// Assign this value.
	Default(Value),
	/// Compute the value by running a script.
	Computed {
		/// The script source, receiving the field's current [`Value`]
		/// (possibly null) as `input` and returning the new value, which is
		/// validated against the field's schema before assignment.
		script: String,
	},
}

impl Default for OnMissing {
	/// [`OnMissing::Default`] of [`Value::Null`], the field-binding seed: a
	/// widget bound to an absent field starts null rather than erroring.
	fn default() -> Self { Self::Default(Value::Null) }
}

impl OnMissing {
	/// Resolve this policy into the value to assign to `field`.
	///
	/// `input` is the field's current value, `None` when it is absent. The
	/// result is *not* validated here; the caller validates against the field's
	/// schema, since only it knows the schema.
	pub async fn resolve(
		&self,
		field: &str,
		input: Option<&Value>,
	) -> Result<Value> {
		match self {
			Self::Error => {
				bevybail!("no value for field `{field}` and no resolution")
			}
			Self::Default(value) => value.clone().xok(),
			Self::Computed { script } => {
				let _ = (script, input);
				unimplemented!(
					"`OnMissing::Computed` needs the js runtime seam"
				)
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	async fn default_resolves_to_its_value() {
		OnMissing::Default(value!(7))
			.resolve("count", None)
			.await
			.unwrap()
			.xpect_eq(value!(7));
	}

	#[crate::test]
	async fn error_names_the_field() {
		OnMissing::Error
			.resolve("count", None)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("count");
	}

	/// The default policy is the widget seed, so a bare [`FieldRef`] keeps
	/// initializing an absent field with null.
	#[crate::test]
	fn defaults_to_a_null_seed() {
		OnMissing::default().xpect_eq(OnMissing::Default(Value::Null));
	}

	#[crate::test]
	#[should_panic(expected = "js runtime seam")]
	async fn computed_is_unimplemented() {
		OnMissing::Computed {
			script: "input + 1".into(),
		}
		.resolve("count", None)
		.await
		.ok();
	}
}
