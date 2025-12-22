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
		fn parse_inner(
			items: &mut Vec<ParamMeta>,
			type_info: &TypeInfo,
		) -> Result {
			match type_info {
				TypeInfo::Struct(struct_info) => {
					items.extend(struct_info.iter().map(ParamMeta::from_field));
				}
				TypeInfo::Tuple(tuple_info) => {
					for field in tuple_info.iter() {
						parse_inner(
							items,
							field.type_info().ok_or_else(|| {
								bevyhow!("Field has no type info")
							})?,
						)?;
					}
				}
				_ => {
					bevybail!(
						"Failed to parse ParamsPartial, only structs and tuples of structs are allowed"
					)
				}
			}
			Ok(())
		}

		parse_inner(&mut items, T::type_info())?;
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
	description: Option<String>,
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
	pub fn new(name: impl Into<String>, value: ParamValue) -> Self {
		Self {
			value,
			name: name.into(),
			short: None,
			description: None,
			optional: false,
		}
	}

	pub fn from_field(field: &bevy::reflect::NamedField) -> Self {
		#[cfg(feature = "reflect_documentation")]
		let description = field.docs().map(|docs| docs.into());
		#[cfg(not(feature = "reflect_documentation"))]
		let description = None;	
		Self {
			name: field.name().into(),
			description,
			short: None,
			optional: false,
			value: ParamValue::Flag,
		}
	}

	/// The name of the param
	pub fn name(&self) -> &str { &self.name }

	/// The description of the param
	pub fn description(&self) -> Option<&str> { self.description.as_deref() }

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
			ParamMeta::new("zebra", ParamValue::Single),
			ParamMeta::new("alpha", ParamValue::Flag),
			ParamMeta::new("beta", ParamValue::Multiple),
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
			ParamMeta {
				short: Some('f'),
				..ParamMeta::new("foo", ParamValue::Flag)
			},
			ParamMeta {
				optional: true,
				..ParamMeta::new("bar", ParamValue::Single)
			},
			ParamMeta {
				short: Some('f'),
				..ParamMeta::new("foo", ParamValue::Flag)
			},
			ParamMeta::new("baz", ParamValue::Multiple),
			ParamMeta {
				optional: true,
				..ParamMeta::new("bar", ParamValue::Single)
			},
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
		let flag = ParamMeta::new("flag", ParamValue::Flag);
		let single = ParamMeta::new("single", ParamValue::Single);
		let multiple = ParamMeta::new("multi", ParamValue::Multiple);

		flag.value().xpect_eq(ParamValue::Flag);
		single.value().xpect_eq(ParamValue::Single);
		multiple.value().xpect_eq(ParamValue::Multiple);
	}

	#[test]
	fn meta_optional_variants() {
		let required = ParamMeta::new("req", ParamValue::Single);
		let optional = ParamMeta {
			optional: true,
			..ParamMeta::new("opt", ParamValue::Single)
		};

		required.optional().xpect_false();
		optional.optional().xpect_true();
	}

	#[test]
	fn meta_with_short() {
		let with_short = ParamMeta {
			short: Some('v'),
			..ParamMeta::new("verbose", ParamValue::Flag)
		};
		let without_short = ParamMeta::new("quiet", ParamValue::Flag);

		with_short.short().xpect_eq(Some('v'));
		without_short.short().xpect_eq(None);
	}

	#[test]
	fn partial_items() {
		let metas = vec![ParamMeta::new("foo", ParamValue::Flag), ParamMeta {
			short: Some('b'),
			optional: true,
			..ParamMeta::new("bar", ParamValue::Single)
		}];

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
			ParamMeta::new("foo", ParamValue::Flag),
			ParamMeta::new("foo", ParamValue::Single),
		];

		ParamsPattern::from_metas(metas).xpect_err();
	}

	#[test]
	fn conflict_different_optional() {
		let metas =
			vec![ParamMeta::new("bar", ParamValue::Single), ParamMeta {
				optional: true,
				..ParamMeta::new("bar", ParamValue::Single)
			}];

		ParamsPattern::from_metas(metas).xpect_err();
	}

	#[test]
	fn conflict_different_short() {
		let metas = vec![
			ParamMeta {
				short: Some('b'),
				..ParamMeta::new("baz", ParamValue::Flag)
			},
			ParamMeta {
				short: Some('z'),
				..ParamMeta::new("baz", ParamValue::Flag)
			},
		];

		ParamsPattern::from_metas(metas).xpect_err();
	}

	#[test]
	fn conflict_different_description() {
		let metas = vec![
			ParamMeta {
				description: Some("first description".into()),
				..ParamMeta::new("qux", ParamValue::Multiple)
			},
			ParamMeta {
				description: Some("second description".into()),
				..ParamMeta::new("qux", ParamValue::Multiple)
			},
		];

		ParamsPattern::from_metas(metas).xpect_err();
	}

	#[test]
	fn no_conflict_identical_params() {
		let metas = vec![
			ParamMeta {
				short: Some('s'),
				optional: true,
				..ParamMeta::new("same", ParamValue::Flag)
			},
			ParamMeta {
				short: Some('s'),
				optional: true,
				..ParamMeta::new("same", ParamValue::Flag)
			},
		];

		let pattern = ParamsPattern::from_metas(metas).unwrap();
		pattern.items.len().xpect_eq(1);
		pattern.items[0].name().xpect_eq("same");
	}

	#[test]
	fn conflict_multiple_params() {
		let metas = vec![
			ParamMeta::new("alpha", ParamValue::Flag),
			ParamMeta::new("beta", ParamValue::Single),
			ParamMeta {
				description: Some("conflicting".into()),
				..ParamMeta::new("beta", ParamValue::Single)
			},
			ParamMeta::new("gamma", ParamValue::Multiple),
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
