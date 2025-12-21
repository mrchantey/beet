use crate::prelude::*;
use beet_core::prelude::*;




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
	/// deduplicates the metas and creates a canonical [`ParamsPattern`]
	pub fn from_metas(mut items: Vec<ParamMeta>) -> Self {
		items.sort_by(|a, b| a.name.cmp(&b.name));
		items.dedup();
		Self { items }
	}

	/// [`Self::Collect`] represented as a bevy system
	pub fn collect_system(
		entity: In<Entity>,
		query: RouteQuery,
	) -> ParamsPattern {
		Self::collect(*entity, &query)
	}

	/// Collects a [`ParamsPattern`] for a provided entity.
	/// Only the provided entity and its parents are checked, any sibling
	/// middleware params should also be specified at the [`Endpoint`].
	pub fn collect(entity: Entity, query: &RouteQuery) -> ParamsPattern {
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
