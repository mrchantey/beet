use crate::prelude::*;
use beet_core::prelude::*;

/// Marker disambiguating the [`TypedFieldRef`] markup-read [`IntoNode`] impl.
pub struct NodeTypedFieldRefMarker;

/// Read a [`TypedFieldRef`] in markup, ie `rsx!{ <span>{count}</span> }`,
/// lowering to the inner [`FieldRef`] that syncs on `Changed<Document>`.
impl<T> IntoNode<(NotNodeBundleMarker, NodeTypedFieldRefMarker)>
	for TypedFieldRef<T>
{
	fn into_node(self) -> impl Bundle { self.field() }
}
