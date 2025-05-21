use crate::prelude::*;
use bevy::ecs::bundle::Bundle;


/// Blanket Bundle trait called on all block nodes and attributes.
/// allowing primitives to be converted into a bundle,
/// for example, [`String`] into [`TextNode`].
///
/// ```rust ignore
/// // the following
/// rsx!{{"howdy"}}
/// // becomes
/// "howdy".into_bundle()
/// ```
pub trait IntoBundle<M> {
	/// Called for nodes and attributes:
	/// `rsx!{"howdy"}` becomes `TextNode::new("howdy")`
	/// `rsx!{<span {"howdy"} />}` becomes `TextNode::new("howdy")`
	fn into_node_bundle(self) -> impl Bundle;
	/// Called for attribute keys:
	/// `rsx!{<span {"hidden"}="true" />}` becomes `AttributeKey::new("hidden")`
	///
	/// For primitives this will also insert an [`AttributeKeyStr`]
	fn into_attr_key_bundle(self) -> impl Bundle;
	/// Called for attribute values:
	/// `rsx!{<span hidden={true} />}` becomes `AttributeValue::new(true)`
	///
	/// For primitives this will also insert an [`AttributeValueStr`]
	fn into_attr_val_bundle(self) -> impl Bundle;
}

impl<T: Bundle> IntoBundle<T> for T {
	fn into_node_bundle(self) -> impl Bundle { self }
	fn into_attr_key_bundle(self) -> impl Bundle { AttributeValue(self) }
	fn into_attr_val_bundle(self) -> impl Bundle { AttributeValue(self) }
}

pub struct IntoTextNodeBundleMarker;

impl IntoBundle<IntoTextNodeBundleMarker> for &str {
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
			impl IntoBundle<IntoTextNodeBundleMarker> for $t {
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
primitives_into_bundle!(
	String, bool, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128,
	isize, f32, f64
);
