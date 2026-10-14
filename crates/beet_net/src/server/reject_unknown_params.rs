//! The strict url space: a request carrying a param nothing on its route
//! declares is refused rather than silently skipped.
use crate::prelude::*;
use beet_core::prelude::*;

/// Refuse, with a `400`, a request carrying a param its route does not take,
/// naming each and the declared param it most plausibly misspells:
/// `deploy --elevate` answers `did you mean --elevated?` rather than running a
/// plain deploy.
///
/// What a route takes is what its [`ParamsPattern`] holds: every param a
/// reader running for it declares by taking it, the route's own and its
/// steps' ([`RouteParams`]) and those of the middleware and servers above it
/// ([`MiddlewareParams`]). A route taking `rest: Params<Rest>` refuses nothing.
///
/// Most servers must tolerate params they never read: a page is linked with
/// `?utm_source=..` and served to a browser that appends whatever it likes. A
/// command line is the opposite case, where a misspelt flag silently ignored is
/// a different command run, so every [`CliServer`] is strict by requirement.
///
/// It governs the url space it heads: the router it sits on, or the one
/// beneath the server it sits on, never a router nested further down, so a
/// site a strict CLI serves over http (`serve --server=http`) still accepts
/// `?utm_source`. An unmatched path in a strict url space is refused the same
/// way when it misspells a route, so `serv` answers `did you mean serve?`.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RejectUnknownParams;

/// Whether a url space is governed by a [`RejectUnknownParams`], the cheap
/// half of the check, made by a router on every request it routes.
#[derive(SystemParam)]
pub struct RejectUnknownParamsQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	policies: Query<'w, 's, (), With<RejectUnknownParams>>,
	paths: Query<'w, 's, &'static PathPartial>,
}

impl RejectUnknownParamsQuery<'_, '_> {
	/// Whether the url space `root` heads rejects unknown params: a
	/// [`RejectUnknownParams`] on it, or on an ancestor below the next
	/// enclosing url space root ([`PathPartial::root`]).
	pub fn governs(&self, root: Entity) -> bool {
		self.ancestors
			.iter_ancestors_inclusive(root)
			.enumerate()
			.take_while(|(index, entity)| {
				*index == 0
					|| !self.paths.get(*entity).is_ok_and(|path| path.is_root)
			})
			.any(|(_, entity)| self.policies.contains(entity))
	}
}
