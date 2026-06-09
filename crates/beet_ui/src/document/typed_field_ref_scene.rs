use crate::prelude::*;
use beet_core::prelude::*;

/// Marker disambiguating the [`TypedFieldRef`] markup-read [`IntoScene`] impl.
pub struct SceneTypedFieldRefMarker;

/// Read a [`TypedFieldRef`] in markup, ie `rsx!{ <span>{count}</span> }`,
/// lowering to the inner [`FieldRef`] that syncs on `Changed<Document>`.
impl<T> IntoScene<(NotSceneMarker, SceneTypedFieldRefMarker)>
	for TypedFieldRef<T>
{
	fn into_scene(self) -> impl bevy::scene::Scene {
		self.field().into_scene()
	}
}
