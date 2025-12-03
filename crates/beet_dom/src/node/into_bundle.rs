use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::ecs::spawn::SpawnIter;
use bevy::ecs::system::IntoObserverSystem;

/// A superset of [`Bundle`], allowing primitives to be wrapped
/// in a [`TextNode`] and entities to be reparented.
///
/// ```rust ignore
/// // the following
/// rsx!{{"howdy"}}
/// // becomes
/// "howdy".into_bundle()
/// ```
pub trait IntoBundle<M> {
	/// Called for nodes and attributes expressions:
	/// `rsx!{"howdy"}` becomes `TextNode::new("howdy")`
	/// `rsx!{<span {"howdy"} />}` becomes `TextNode::new("howdy")`
	fn into_bundle(self) -> impl Bundle;
}
pub struct BundleMarker;

impl<T: Bundle> IntoBundle<BundleMarker> for T {
	fn into_bundle(self) -> impl Bundle { self }
}

#[extend::ext(name = AnyBundleExt)]
pub impl<T, M> T
where
	T: IntoBundle<M>,
{
	/// Converts the bundle to be inserted via [`OnSpawn`], allowing branches
	/// to return the same type.
	///
	/// ## Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_dom::prelude::*;
	///
	/// let bundle = if true {
	/// 	().any_bundle()
	/// } else {
	/// 	Name::new("foo").any_bundle()
	/// };
	///```
	fn any_bundle(self) -> OnSpawn {
		let bundle = self.into_bundle();
		OnSpawn::new(move |entity| {
			entity.insert(bundle);
		})
	}
}

#[extend::ext(name=AnyBundleCloneExt)]
pub impl<T, M> T
where
	T: 'static + Send + Sync + Clone + IntoBundle<M>,
{
	/// Like [`AnyBundleExt::any_bundle`] but returns a [`Clone`]
	/// type.
	fn any_bundle_clone(self) -> OnSpawnClone {
		OnSpawnClone::insert(move || self.clone().into_bundle())
	}
}


pub struct ObserverMarker;

impl<T, E, B: Bundle, M> IntoBundle<(ObserverMarker, E, B, M)> for T
where
	E: Event,
	B: Bundle,
	T: 'static + Send + Sync + IntoObserverSystem<E, B, M>,
{
	fn into_bundle(self) -> impl Bundle {
		(EventTarget, OnSpawn::observe(self))
	}
}

impl<T, M> IntoBundle<(Self, M)> for Option<T>
where
	T: IntoBundle<M>,
{
	fn into_bundle(self) -> impl Bundle {
		match self {
			Some(item) => item.any_bundle(),
			None => OnSpawn::new(|_| {}),
		}
	}
}
impl IntoBundle<Self> for RoutePath {
	fn into_bundle(self) -> impl Bundle { TextNode::new(self.to_string()) }
}


impl<T, M> IntoBundle<(Self, M)> for Vec<T>
where
	T: 'static + Send + Sync + IntoBundle<M>,
{
	fn into_bundle(self) -> impl Bundle {
		(
			FragmentNode,
			Children::spawn(SpawnIter(
				self.into_iter().map(|item| item.into_bundle()),
			)),
		)
	}
}

/// Entities are convert into children of the entity they are inserted into.
///
/// `rsx!{<div>{entity}</div>}` spawns an entity with this OnSpawn effect,
/// which becomes the parent of the entity passed in.
impl IntoBundle<Self> for Entity {
	fn into_bundle(self) -> impl Bundle {
		OnSpawnTyped::new(move |spawned_entity| {
			spawned_entity.insert(FragmentNode);
			let id = spawned_entity.id();
			spawned_entity.world_scope(|world| {
				world.entity_mut(self).insert(ChildOf(id));
			});
		})
	}
}

impl IntoBundle<Self> for String {
	fn into_bundle(self) -> impl Bundle { TextNode::new(self) }
}
impl IntoBundle<Self> for &String {
	fn into_bundle(self) -> impl Bundle { TextNode::new(self.clone()) }
}
impl IntoBundle<Self> for &str {
	fn into_bundle(self) -> impl Bundle { TextNode::new(self.to_string()) }
}

impl IntoBundle<Self> for bool {
	fn into_bundle(self) -> impl Bundle {
		(TextNode::new(self.to_string()), BoolNode::new(self))
	}
}

impl IntoBundle<Self> for f32 {
	fn into_bundle(self) -> impl Bundle {
		let value = self as f64;
		(TextNode::new(value.to_string()), NumberNode::new(value))
	}
}

impl IntoBundle<Self> for f64 {
	fn into_bundle(self) -> impl Bundle {
		(TextNode::new(self.to_string()), NumberNode::new(self))
	}
}

impl IntoBundle<Self> for u8 {
	fn into_bundle(self) -> impl Bundle {
		let value = self as f64;
		(TextNode::new(value.to_string()), NumberNode::new(value))
	}
}

impl IntoBundle<Self> for u16 {
	fn into_bundle(self) -> impl Bundle {
		let value = self as f64;
		(TextNode::new(value.to_string()), NumberNode::new(value))
	}
}

impl IntoBundle<Self> for u32 {
	fn into_bundle(self) -> impl Bundle {
		let value = self as f64;
		(TextNode::new(value.to_string()), NumberNode::new(value))
	}
}

impl IntoBundle<Self> for u64 {
	fn into_bundle(self) -> impl Bundle {
		let value = self as f64;
		(TextNode::new(value.to_string()), NumberNode::new(value))
	}
}

// impl IntoBundle<Self> for u128 {
// 	fn into_bundle(self) -> impl Bundle {
// 		let value = self as f64;
// 		(TextNode::new(value.to_string()), NumberNode::new(value))
// 	}
// }

impl IntoBundle<Self> for usize {
	fn into_bundle(self) -> impl Bundle {
		let value = self as f64;
		(TextNode::new(value.to_string()), NumberNode::new(value))
	}
}

impl IntoBundle<Self> for i8 {
	fn into_bundle(self) -> impl Bundle {
		let value = self as f64;
		(TextNode::new(value.to_string()), NumberNode::new(value))
	}
}

impl IntoBundle<Self> for i16 {
	fn into_bundle(self) -> impl Bundle {
		let value = self as f64;
		(TextNode::new(value.to_string()), NumberNode::new(value))
	}
}

impl IntoBundle<Self> for i32 {
	fn into_bundle(self) -> impl Bundle {
		let value = self as f64;
		(TextNode::new(value.to_string()), NumberNode::new(value))
	}
}

impl IntoBundle<Self> for i64 {
	fn into_bundle(self) -> impl Bundle {
		let value = self as f64;
		(TextNode::new(value.to_string()), NumberNode::new(value))
	}
}

// impl IntoBundle<Self> for i128 {
// 	fn into_bundle(self) -> impl Bundle {
// 		let value = self as f64;
// 		(TextNode::new(value.to_string()), NumberNode::new(value))
// 	}
// }

impl IntoBundle<Self> for isize {
	fn into_bundle(self) -> impl Bundle {
		let value = self as f64;
		(TextNode::new(value.to_string()), NumberNode::new(value))
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn works() {
		fn is_bundle<M>(_: impl IntoBundle<M>) {}
		#[derive(Event)]
		struct Foo;
		is_bundle(|_: On<Foo>| {});
	}
}
