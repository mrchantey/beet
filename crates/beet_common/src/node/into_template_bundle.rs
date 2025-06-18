use crate::prelude::*;
use beet_bevy::prelude::BundleIter;
use beet_bevy::prelude::EntityObserver;
use bevy::ecs::bundle::Bundle;
use bevy::ecs::event::Event;
use bevy::ecs::system::IntoObserverSystem;

/// Very inclusive version of [`Bundle`], accounting for the location
/// of the bundle in a tree, and allowing primitives to be wrapped
/// in a [`TextNode`].
/// for example, [`String`] into [`TextNode`].
///
/// ```rust ignore
/// // the following
/// rsx!{{"howdy"}}
/// // becomes
/// "howdy".into_node_bundle()
/// ```
pub trait IntoTemplateBundle<M> {
	/// Called for nodes and attributes expressions:
	/// `rsx!{"howdy"}` becomes `TextNode::new("howdy")`
	/// `rsx!{<span {"howdy"} />}` becomes `TextNode::new("howdy")`
	/// This is also called by default in [`Self::into_attribute_bundle`],
	/// wrapping them in [`AttributeKey`] and [`AttributeValue`].
	fn into_node_bundle(self) -> impl Bundle;
	/// By default calls [`Self::into_node_bundle`], but can be overridden,
	/// for instance literals like `String` or `bool` will insert
	/// an [`AttributeLit`] instead.
	fn into_attribute_bundle(self) -> impl Bundle
	where
		Self: 'static + Send + Sync + Sized,
	{
		self.into_node_bundle()
	}
}
pub struct BundleMarker;

impl<T: Bundle> IntoTemplateBundle<(T, BundleMarker)> for T {
	fn into_node_bundle(self) -> impl Bundle { self }
}

/// Observers
pub struct ObserverMarker;

impl<T, E, B: Bundle, M> IntoTemplateBundle<(ObserverMarker, E, B, M)> for T
where
	E: Event,
	B: Bundle,
	T: IntoObserverSystem<E, B, M>,
{
	fn into_node_bundle(self) -> impl Bundle { EntityObserver::new(self) }
}


// includes Option
pub struct IterMarker;

impl<I: IntoIterator<Item = B>, B, M> IntoTemplateBundle<(IterMarker, M)> for I
where
	B: IntoTemplateBundle<M>,
	I::IntoIter: 'static + Send + Sync + Iterator<Item = B>,
{
	fn into_node_bundle(self) -> impl Bundle {
		let bundle_iter = self.into_iter().map(|item| item.into_node_bundle());
		BundleIter::new(bundle_iter)
	}
}

pub struct IntoTextNodeBundleMarker;

macro_rules! primitives_into_bundle {
	($($t:ty),*) => {
		$(
			impl IntoTemplateBundle<IntoTextNodeBundleMarker> for $t {
				fn into_node_bundle(self) -> impl Bundle { TextNode::new(self.to_string()) }
				fn into_attribute_bundle(self) -> impl Bundle {
					AttributeLit::new(self)
				}
			}
		)*
	}
}

// Implement for primitives
#[rustfmt::skip]
primitives_into_bundle!(
	&str,String, bool, 
	f32, f64,
	u8, u16, u32, u64, u128, usize, 
	i8, i16, i32, i64, i128, isize
);



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;

	#[test]
	fn works() {
		fn is_bundle<M>(_: impl IntoTemplateBundle<M>) {}
		#[derive(Event)]
		struct Foo;
		is_bundle(|_: Trigger<Foo>| {});
	}
}
