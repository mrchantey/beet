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

	/// Whether this node is a page the public may reach: a [`PageRoute`] at a
	/// fully static path answering `GET`, and not a draft in a production
	/// build.
	///
	/// The ONE visibility predicate a static export, a sitemap, a feed and a
	/// search index share, so a site can never export a page it does not list
	/// or list one it does not export.
	/// [`Unlisted`](beet_ui::prelude::PageVisibility::Unlisted) pages pass
	/// here — they serve and export like any other — and each syndication route
	/// drops them itself with [`PageMeta::is_listed`].
	///
	/// [`PageRoute`]: crate::prelude::PageRoute
	// Takes the metadata rather than reading it, so the same predicate serves a
	// `&World` caller (static export) and a `Query` one (the syndication
	// routes). std-only: `PageMeta` is a beet_ui type, and a no_std router has
	// no scene pipeline to render a page with.
	#[cfg(feature = "std")]
	pub fn is_public_page(
		&self,
		meta: Option<&beet_ui::prelude::PageMeta>,
		is_prod: bool,
	) -> bool {
		use beet_ui::prelude::PageMeta;
		self.is_page_route
			&& self.path.is_static()
			&& !self.method.is_some_and(|method| method != HttpMethod::Get)
			&& !(is_prod && meta.is_some_and(PageMeta::is_draft))
	}

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
