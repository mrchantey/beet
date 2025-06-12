use crate::prelude::*;
use bevy::ecs::bundle::Bundle;
use beet_bevy::prelude::Maybe;

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
	/// This is also called by default in [`Self::into_attr_key_bundle`] and
	/// [`Self::into_attr_val_bundle`],
	/// wrapping them in [`AttributeKey`] and [`AttributeValue`].
	fn into_node_bundle(self) -> impl Bundle;
	/// Called for attribute keys, wrapping `Self` in an [`AttributeKey`].
	/// for primitives this will also insert an [`AttributeKeyStr`]
	fn into_attr_key_bundle(self) -> impl Bundle
	where
		Self: 'static + Send + Sync + Sized,
	{
		AttributeKey(self)
	}
	/// Called for attribute values, wrapping `Self` in an [`AttributeValue`].
	/// for primitives this will also insert an [`AttributeValueStr`]
	fn into_attr_val_bundle(self) -> impl Bundle
	where
		Self: 'static + Send + Sync + Sized,
	{
		AttributeValue(self)
	}
}

impl<T: Bundle> IntoTemplateBundle<T> for T {
	fn into_node_bundle(self) -> impl Bundle { self }
}

pub struct OptionBundleMarker;


impl<T: IntoTemplateBundle<M>, M> IntoTemplateBundle<(OptionBundleMarker, M)>
	for Option<T>
{
	fn into_node_bundle(self) -> impl Bundle {
		Maybe(self.map(|val| val.into_node_bundle()))
	}
}



pub struct IntoTextNodeBundleMarker;

impl IntoTemplateBundle<IntoTextNodeBundleMarker> for &str {
	fn into_node_bundle(self) -> impl Bundle { TextNode::new(self.to_string()) }
	fn into_attr_key_bundle(self) -> impl Bundle {
		(
			AttributeKeyStr::new(self.to_string()),
			AttributeKey::new(self.to_string()),
		)
	}
	fn into_attr_val_bundle(self) -> impl Bundle {
		(
			AttributeValueStr::new(self.to_string()),
			AttributeValue::new(self.to_string()),
		)
	}
}

macro_rules! primitives_into_bundle {
	($($t:ty),*) => {
		$(
			impl IntoTemplateBundle<IntoTextNodeBundleMarker> for $t {
				fn into_node_bundle(self) -> impl Bundle { TextNode::new(self.to_string()) }
				fn into_attr_key_bundle(self) -> impl Bundle {
					(AttributeKeyStr::new(self.to_string()), AttributeKey::new(self))
				}
				fn into_attr_val_bundle(self) -> impl Bundle {
					(AttributeValueStr::new(self.to_string()), AttributeValue::new(self))
				}
			}
		)*
	}
}

// Implement for primitives
#[rustfmt::skip]
primitives_into_bundle!(
	String, bool, 
	f32, f64,
	u8, u16, u32, u64, u128, usize, 
	i8, i16, i32, i64, i128, isize
);
