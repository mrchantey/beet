use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;

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
pub struct HelpParams {
	#[reflect(@ParamOptions::desc("Get help"))]
	help: bool,
}


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
	/// Creates a new [`ParamsPartial`] from a type that implements [`Reflect`].
	/// Accepted types are structs and tuples of structs.
	///
	/// ## Panics
	///
	/// Panics if a non-struct is passed in or fields are missing TypeInfo
	pub fn new<T: Typed>() -> Self {
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
					// recursively add for each item in the tuple
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

		parse_inner(&mut items, T::type_info()).unwrap();
		Self { items }
	}
}

/// The param equivelent of a [`PathPattern`], denoting
/// all params used for this endpoint and its ancestors
#[derive(
	Debug,
	Default,
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
					"conflicting param definitions for '{name}': \nFirst: {first:#?} \nSecond: {second:#?}",
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
	/// The kind of param value
	value: ParamValue,
	/// Additional details for the param
	options: ParamOptions,
	/// Whether specifying the param is required, usually inferred
	/// by presence of `Option`
	required: bool,
}

impl std::fmt::Display for ParamMeta {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Param - name: {}", self.name)?;
		if let Some(short) = self.short() {
			write!(f, ", short: -{}", short)?;
		}
		write!(
			f,
			", required: {}, kind: {}",
			self.is_required(),
			self.value
		)?;
		if let Some(desc) = self.description() {
			write!(f, ", description: {}", desc)?;
		}
		Ok(())
	}
}


impl ParamMeta {
	/// Create a new `ParamMeta`
	pub fn new(name: impl Into<String>, value: ParamValue) -> Self {
		Self {
			value,
			name: name.into(),
			options: default(),
			required: false,
		}
	}

	pub fn from_field(field: &bevy::reflect::NamedField) -> Self {
		let value = ParamValue::from_type_path(field.type_path());
		let required = match value {
			ParamValue::Single => {
				!field.type_path().starts_with("core::option::Option<")
			}
			_ => false,
		};

		Self {
			name: field.name().into(),
			value: ParamValue::from_type_path(field.type_path()),
			options: ParamOptions::from_reflect(field),
			required,
		}
	}

	/// The name of the param
	pub fn name(&self) -> &str { &self.name }

	/// The description of the param
	pub fn description(&self) -> Option<&str> {
		self.options.description.as_deref()
	}

	pub fn required(mut self) -> Self {
		self.required = true;
		self
	}

	pub fn with_short(mut self, short: char) -> Self {
		self.options.short = Some(short);
		self
	}
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.options.description = Some(description.into());
		self
	}

	/// The short character representation
	pub fn short(&self) -> Option<char> { self.options.short }

	/// Whether the param is required
	pub fn is_required(&self) -> bool { self.required }

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

impl std::fmt::Display for ParamValue {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Flag => write!(f, "flag"),
			Self::Single => write!(f, "single"),
			Self::Multiple => write!(f, "multiple"),
		}
	}
}

impl ParamValue {
	fn from_type_path(type_path: &str) -> Self {
		match type_path {
			"bool" => Self::Flag,
			val if val.starts_with("alloc::vec::Vec") => Self::Multiple,
			_ => Self::Single,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub struct ParamOptions {
	/// A description of the route param, usually the
	/// docs section of a provided params type
	description: Option<String>,
	/// requiredly specify a single character representation
	/// for a route param
	short: Option<char>,
}

impl Default for ParamOptions {
	fn default() -> Self {
		Self {
			description: None,
			short: None,
		}
	}
}

impl ParamOptions {
	pub fn desc(description: impl Into<String>) -> Self {
		Self {
			description: Some(description.into()),
			short: None,
		}
	}

	pub fn desc_and_short(description: impl Into<String>, short: char) -> Self {
		Self {
			description: Some(description.into()),
			short: Some(short),
		}
	}

	pub fn short(short: char) -> Self {
		Self {
			description: None,
			short: Some(short),
		}
	}


	fn from_reflect(field: &bevy::reflect::NamedField) -> Self {
		let opts = field.get_attribute::<Self>().cloned().unwrap_or_default();
		// Override description from docs if not specified
		#[cfg(feature = "reflect_documentation")]
		if opts.description == None {
			opts = field.docs().map(|docs| docs.into());
		}
		opts
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn pattern_deduplication() {
		let metas = vec![
			ParamMeta::new("foo", ParamValue::Flag),
			ParamMeta::new("bar", ParamValue::Single),
			ParamMeta::new("foo", ParamValue::Flag),
			ParamMeta::new("baz", ParamValue::Multiple),
			ParamMeta::new("bar", ParamValue::Single),
		];

		let pattern = ParamsPattern::from_metas(metas).unwrap();
		pattern.items.len().xpect_eq(3);
		pattern.items[0].name().xpect_eq("bar");
		pattern.items[1].name().xpect_eq("baz");
		pattern.items[2].name().xpect_eq("foo");
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
	fn conflict_different_required() {
		let metas = vec![
			ParamMeta::new("bar", ParamValue::Single),
			ParamMeta::new("bar", ParamValue::Single).required(),
		];

		ParamsPattern::from_metas(metas).xpect_err();
	}

	#[test]
	fn conflict_different_short() {
		let metas = vec![
			ParamMeta::new("baz", ParamValue::Flag).with_short('b'),
			ParamMeta::new("baz", ParamValue::Flag).with_short('z'),
		];

		ParamsPattern::from_metas(metas).xpect_err();
	}

	#[test]
	fn conflict_different_description() {
		let metas = vec![
			ParamMeta::new("qux", ParamValue::Multiple)
				.with_description("first description"),
			ParamMeta::new("qux", ParamValue::Multiple)
				.with_description("second description"),
		];

		ParamsPattern::from_metas(metas).xpect_err();
	}

	#[test]
	fn no_conflict_identical_params() {
		let metas = vec![
			ParamMeta::new("same", ParamValue::Flag)
				.with_short('s')
				.required(),
			ParamMeta::new("same", ParamValue::Flag)
				.with_short('s')
				.required(),
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
			ParamMeta::new("beta", ParamValue::Single)
				.with_description("conflicting"),
			ParamMeta::new("gamma", ParamValue::Multiple),
		];

		ParamsPattern::from_metas(metas).xpect_err();
	}

	#[test]
	fn from_reflect() {
		#[derive(Reflect)]
		struct MyParams {
			foo: u32,
			#[reflect(@ParamOptions::desc("all about 'bar'"))]
			bar: Option<String>,
			#[reflect(@ParamOptions::desc_and_short("all about 'bazz'",'b'))]
			bazz: Vec<f64>,
			boo: bool,
		}

		ParamsPartial::new::<MyParams>().xpect_eq(ParamsPartial {
			items: vec![
				ParamMeta::new("foo", ParamValue::Single).required(),
				ParamMeta::new("bar", ParamValue::Single)
					.with_description("all about 'bar'"),
				ParamMeta::new("bazz", ParamValue::Multiple)
					.with_description("all about 'bazz'")
					.with_short('b'),
				// .required(),
				ParamMeta::new("boo", ParamValue::Flag),
			],
		});
	}
}
