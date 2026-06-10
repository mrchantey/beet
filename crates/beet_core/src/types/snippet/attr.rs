//! Block-attribute helpers the `rsx!` / `#[template]` lowerings reach for:
//! a dynamic attribute and an optional attribute.
use crate::prelude::*;

/// Build a single markup attribute as a [`Bundle`], for use as an `rsx!` block
/// attribute: `<a {attr("href", url)}/>`. Attributes accumulate, so this sits
/// alongside the element's literal attributes rather than replacing them.
///
/// It spawns the attribute as a *related* entity ([`AttributeOf`] the element)
/// rather than `related!`-setting the whole [`Attributes`] target, so multiple
/// block attributes and the element's literal attributes all coexist instead of
/// the last one clobbering the rest.
///
/// Pair it with [`Option`] for an attribute that disappears when absent — see
/// [`optional_attr`], the ergonomic form for optional props.
pub fn attr(key: impl Into<String>, value: impl Into<Value>) -> impl Bundle {
	let key = key.into();
	let value = value.into();
	OnSpawn::new(move |entity| {
		let element = entity.id();
		entity.world_scope(move |world| {
			world.spawn((AttributeOf::new(element), Attribute::new(key), value));
		});
	})
}

/// A markup attribute that renders only when its value is [`Some`], for use as an
/// `rsx!` block attribute: `<input {optional_attr("name", name)}/>` where
/// `name: Option<String>`.
///
/// A [`None`] renders nothing — unlike a defaulted empty string, which would
/// emit an incorrect `name=""`. This is the ergonomic answer to "this prop is
/// optional, so its attribute should be absent when unset".
pub fn optional_attr(
	key: impl Into<String>,
	value: Option<impl Into<Value>>,
) -> impl Bundle {
	OnSpawn::insert_option(value.map(|value| attr(key, value)))
}
