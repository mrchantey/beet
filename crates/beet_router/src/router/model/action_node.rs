//! The node at a route: what the tree holds at each of its paths, and the query
//! tuple a rebuild collects one from.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The output handle of a scene route: a newtype over the render-root
/// [`Entity`].
///
/// A dedicated type (rather than a bare `Entity`) is required so the
/// `IntoResponseWithRequestParts` impl does not collide with the blanket `Serialize`
/// impl — `Entity` is itself `Serialize`. The render side (the impl, the
/// despawn list) lives in `scene_routes`; the type itself is here in the
/// no_std core so [`ActionNode::is_scene`] can detect scene routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest(pub Entity);

/// An action route node, representing a callable action at a specific path.
/// Scene routes are identified by their output type being [`PageRequest`].
#[derive(Debug, Clone)]
pub struct ActionNode {
	/// The entity containing this action.
	pub entity: Entity,
	/// Metadata about the action's input/output types.
	pub meta: ActionMeta,
	/// The parameter pattern for this action.
	pub params: ParamsPattern,
	/// The full path pattern for this action.
	pub path: PathPattern,
	/// Optional HTTP method restriction.
	pub method: Option<HttpMethod>,
	/// Whether the route carries the [`PageRoute`] marker, ie a user-facing page
	/// route rather than an infrastructure or data route. Drives inclusion in the
	/// navigation [`RouteSidebar`](crate::prelude::RouteSidebar).
	pub is_page_route: bool,
}

impl ActionNode {
	/// Whether this action is a scene route (output type is [`PageRequest`]).
	pub fn is_scene(&self) -> bool { self.meta.output_is::<PageRequest>() }

	/// The action's description from doc comments, if available.
	pub fn description(&self) -> Option<&str> { self.meta.description() }

	/// Merge the dynamic path segments matched by this node's [`PathPattern`]
	/// into the request params, so handlers can read a `:id` value via
	/// [`RequestParts::get_param`] or the [`QueryParams`] extractor.
	///
	/// Path params take precedence over query params on key collision.
	/// A no-op when the request path does not match this node's pattern.
	pub fn merge_path_params(&self, request: &mut Request) {
		let Ok(path_match) = self.path.parse_path(request.path()) else {
			return;
		};
		let params = request.params_mut();
		for (key, values) in path_match.dyn_map.into_iter_all() {
			// path params win over query params on collision
			params.remove(&key);
			if values.is_empty() {
				params.insert_key(key);
			} else {
				params.insert_vec(key, values);
			}
		}
	}
}

/// The query tuple type used to collect action components for [`ActionNode::from_query`].
pub(crate) type ActionQueryItem<'a> = (
	Entity,
	&'a ActionMeta,
	&'a PathPattern,
	&'a ParamsPattern,
	Option<&'a HttpMethod>,
	Has<PageRoute>,
);

impl ActionNode {
	/// Create an [`ActionNode`] from a fetched [`ActionQueryItem`]. Takes the
	/// query item shape (`Has<PageRoute>` resolves to a `bool`), not the
	/// query-data alias itself.
	pub fn from_query(
		(entity, meta, path, params, method, is_page_route): (
			Entity,
			&ActionMeta,
			&PathPattern,
			&ParamsPattern,
			Option<&HttpMethod>,
			bool,
		),
	) -> Self {
		Self {
			entity,
			meta: meta.clone(),
			params: params.clone(),
			path: path.clone(),
			method: method.cloned(),
			is_page_route,
		}
	}
}
