use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;



/// The param equivelent of a [`PathPartial`], denoting
/// all params used at this point in the graph. For the full
/// list see [`Endpoint::params`]
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
pub struct ParamsPartial {
	pub items: Vec<ParamMeta>,
}

impl ParamsPartial {
	fn from_reflect<T: Typed>() -> Result<Self> {
		let mut items = Vec::new();
		match T::type_info() {
			TypeInfo::Struct(struct_info) => todo!(),
			TypeInfo::Map(map_info) => todo!(),
			TypeInfo::Tuple(tuple_info) => todo!(),
			TypeInfo::TupleStruct(tuple_struct_info) => todo!(),
			_ => {
				bevybail!(
					"Failed to parse ParamsPartial, only Struct, Map and tuples of these are allowed"
				)
			}
		}
		Self { items }.xok()
	}
}

/// The param equivelent of a [`PathPattern`], denoting
/// all params used for this endpoint and its ancestors
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
pub struct ParamsPattern {
	/// A list of params for this endpoint,
	/// sorted by name and deduplicated.
	items: Vec<ParamMeta>,
}

impl ParamsPattern {
	/// Deduplicates the metas and creates a canonical [`ParamsPattern`]
	/// ## Errors
	/// - Errors if params with the same name have conflicting definitions
	pub fn from_metas(mut items: Vec<ParamMeta>) -> Result<Self> {
		items.sort_by(|a, b| a.name.cmp(&b.name));

		// check for conflicts before deduplication
		for window in items.windows(2) {
			if window[0].name == window[1].name && window[0] != window[1] {
				bevybail!(
					"conflicting param definitions for '{name}': \nFirst: {first:#?} \nSecond: {second:?}",
					name = window[0].name,
					first = window[0],
					second = window[1],
				);
			}
		}

		items.dedup();
		Self { items }.xok()
	}

	/// [`Self::collect`] represented as a bevy system
	pub fn collect_system(
		entity: In<Entity>,
		query: RouteQuery,
	) -> Result<ParamsPattern> {
		Self::collect(*entity, &query)
	}

	/// Collects a [`ParamsPattern`] for a provided entity.
	/// Only the provided entity and its parents are checked, any sibling
	/// middleware params should also be specified at the [`Endpoint`].
	pub fn collect(
		entity: Entity,
		query: &RouteQuery,
	) -> Result<ParamsPattern> {
		query
			.parents
			// get every PathFilter in ancestors
			.iter_ancestors_inclusive(entity)
			.filter_map(|entity| query.params_partials.get(entity).ok())
			.flat_map(|partial| partial.items.clone())
			.collect::<Vec<_>>()
			.xmap(ParamsPattern::from_metas)
	}
}

/// Metadata for a specific param at a route
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub struct ParamMeta {
	/// The name of the route param, ie the bar in `--bar=3`
	name: String,
	/// A description of the route param, usually the
	/// docs section of a provided params type
	description: String,
	/// Optionally specify a single character representation
	/// for a route param
	short: Option<char>,
	/// Whether specifying the param is required
	optional: bool,
	/// The kind of param value
	value: ParamValue,
}

impl ParamMeta {
	/// Create a new `ParamMeta`
	pub fn new(
		name: impl Into<String>,
		description: impl Into<String>,
		short: Option<char>,
		optional: bool,
		value: ParamValue,
	) -> Self {
		Self {
			name: name.into(),
			description: description.into(),
			short,
			optional,
			value,
		}
	}

	/// The name of the param
	pub fn name(&self) -> &str { &self.name }

	/// The description of the param
	pub fn description(&self) -> &str { &self.description }

	/// The short character representation
	pub fn short(&self) -> Option<char> { self.short }

	/// Whether the param is optional
	pub fn optional(&self) -> bool { self.optional }

	/// The param value type
	pub fn value(&self) -> &ParamValue { &self.value }
}

/// The kind of value a param takes
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum ParamValue {
	/// A simple flag, ie `beet foo --bar`
	Flag,
	/// A single value, ie `beet foo --bar=1`
	Single,
	/// Multiple items allowed, ie `beet foo --bar=1 --bar=2`
	Multiple,
}




#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn pattern_from_metas() {
		let metas = vec![
			ParamMeta::new("zebra", "last", None, false, ParamValue::Single),
			ParamMeta::new("alpha", "first", None, false, ParamValue::Flag),
			ParamMeta::new("beta", "second", None, false, ParamValue::Multiple),
		];

		let pattern = ParamsPattern::from_metas(metas).unwrap();
		pattern.items.len().xpect_eq(3);
		pattern.items[0].name().xpect_eq("alpha");
		pattern.items[1].name().xpect_eq("beta");
		pattern.items[2].name().xpect_eq("zebra");
	}

	#[test]
	fn pattern_deduplication() {
		let metas = vec![
			ParamMeta::new(
				"foo",
				"description",
				Some('f'),
				false,
				ParamValue::Flag,
			),
			ParamMeta::new(
				"bar",
				"description",
				None,
				true,
				ParamValue::Single,
			),
			ParamMeta::new(
				"foo",
				"description",
				Some('f'),
				false,
				ParamValue::Flag,
			),
			ParamMeta::new(
				"baz",
				"description",
				None,
				false,
				ParamValue::Multiple,
			),
			ParamMeta::new(
				"bar",
				"description",
				None,
				true,
				ParamValue::Single,
			),
		];

		let pattern = ParamsPattern::from_metas(metas).unwrap();
		pattern.items.len().xpect_eq(3);
		pattern.items[0].name().xpect_eq("bar");
		pattern.items[1].name().xpect_eq("baz");
		pattern.items[2].name().xpect_eq("foo");
	}

	#[test]
	fn pattern_empty() {
		let pattern = ParamsPattern::from_metas(vec![]).unwrap();
		pattern.items.is_empty().xpect_true();
	}

	#[test]
	fn value_types() {
		let flag =
			ParamMeta::new("flag", "A flag", None, false, ParamValue::Flag);
		let single = ParamMeta::new(
			"single",
			"Single value",
			None,
			false,
			ParamValue::Single,
		);
		let multiple = ParamMeta::new(
			"multi",
			"Multiple values",
			None,
			false,
			ParamValue::Multiple,
		);

		flag.value().xpect_eq(ParamValue::Flag);
		single.value().xpect_eq(ParamValue::Single);
		multiple.value().xpect_eq(ParamValue::Multiple);
	}

	#[test]
	fn meta_optional_variants() {
		let required = ParamMeta::new(
			"req",
			"Required param",
			None,
			false,
			ParamValue::Single,
		);
		let optional = ParamMeta::new(
			"opt",
			"Optional param",
			None,
			true,
			ParamValue::Single,
		);

		required.optional().xpect_false();
		optional.optional().xpect_true();
	}

	#[test]
	fn meta_with_short() {
		let with_short = ParamMeta::new(
			"verbose",
			"Verbose output",
			Some('v'),
			false,
			ParamValue::Flag,
		);
		let without_short = ParamMeta::new(
			"quiet",
			"Quiet mode",
			None,
			false,
			ParamValue::Flag,
		);

		with_short.short().xpect_eq(Some('v'));
		without_short.short().xpect_eq(None);
	}

	#[test]
	fn partial_items() {
		let metas = vec![
			ParamMeta::new("foo", "Foo param", None, false, ParamValue::Flag),
			ParamMeta::new(
				"bar",
				"Bar param",
				Some('b'),
				true,
				ParamValue::Single,
			),
		];

		let partial = ParamsPartial {
			items: metas.clone(),
		};

		partial.items.len().xpect_eq(2);
		partial.items[0].name().xpect_eq("foo");
		partial.items[1].name().xpect_eq("bar");
	}

	#[test]
	fn conflict_different_value_types() {
		let metas = vec![
			ParamMeta::new("foo", "description", None, false, ParamValue::Flag),
			ParamMeta::new(
				"foo",
				"description",
				None,
				false,
				ParamValue::Single,
			),
		];

		ParamsPattern::from_metas(metas).xpect_err();
	}

	#[test]
	fn conflict_different_optional() {
		let metas = vec![
			ParamMeta::new(
				"bar",
				"description",
				None,
				false,
				ParamValue::Single,
			),
			ParamMeta::new(
				"bar",
				"description",
				None,
				true,
				ParamValue::Single,
			),
		];

		ParamsPattern::from_metas(metas).xpect_err();
	}

	#[test]
	fn conflict_different_short() {
		let metas = vec![
			ParamMeta::new(
				"baz",
				"description",
				Some('b'),
				false,
				ParamValue::Flag,
			),
			ParamMeta::new(
				"baz",
				"description",
				Some('z'),
				false,
				ParamValue::Flag,
			),
		];

		ParamsPattern::from_metas(metas).xpect_err();
	}

	#[test]
	fn conflict_different_description() {
		let metas = vec![
			ParamMeta::new(
				"qux",
				"first description",
				None,
				false,
				ParamValue::Multiple,
			),
			ParamMeta::new(
				"qux",
				"second description",
				None,
				false,
				ParamValue::Multiple,
			),
		];

		ParamsPattern::from_metas(metas).xpect_err();
	}

	#[test]
	fn no_conflict_identical_params() {
		let metas = vec![
			ParamMeta::new(
				"same",
				"identical",
				Some('s'),
				true,
				ParamValue::Flag,
			),
			ParamMeta::new(
				"same",
				"identical",
				Some('s'),
				true,
				ParamValue::Flag,
			),
		];

		let pattern = ParamsPattern::from_metas(metas).unwrap();
		pattern.items.len().xpect_eq(1);
		pattern.items[0].name().xpect_eq("same");
	}

	#[test]
	fn conflict_multiple_params() {
		let metas = vec![
			ParamMeta::new("alpha", "first", None, false, ParamValue::Flag),
			ParamMeta::new("beta", "second", None, false, ParamValue::Single),
			ParamMeta::new(
				"beta",
				"conflicting",
				None,
				false,
				ParamValue::Single,
			),
			ParamMeta::new("gamma", "third", None, false, ParamValue::Multiple),
		];

		ParamsPattern::from_metas(metas).xpect_err();
	}

	#[test]
	fn from_reflect() {
		#[derive(Reflect)]
		struct MyParams {
			foo: u32,
			bar: Option<String>,
			bazz: Vec<f64>,
		}
	}
}
