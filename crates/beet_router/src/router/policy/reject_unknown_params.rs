//! The opt-in refusal of a request carrying a param nothing on its dispatch
//! path declares.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Refuse, with a `400`, a request carrying a param nothing on its dispatch
/// path declares, naming each and the declared param it most plausibly
/// misspells: `deploy --elevate` answers `did you mean --elevated?` rather
/// than running a plain deploy.
///
/// Opt-in, because most servers must tolerate params they never read: a page
/// is linked with `?utm_source=..`, and served to a browser that appends
/// whatever it likes. A command line is the opposite case, where a misspelt
/// flag silently ignored is a different command run, so a CLI declares this
/// beside its server:
///
/// ```bsx
/// <CliServer always=true {(CallOnReady, RejectUnknownParams)}>
///     <Router {HelpHandler}>..</Router>
/// </CliServer>
/// ```
///
/// It governs the url space it heads: the [`Router`] it sits on, or the one
/// beneath the server it sits on, never a router nested further down, so a
/// site an opted-in CLI serves over http (`serve --server=http`) still accepts
/// `?utm_source`. Once a request is routed, every param it carries is checked
/// against the [`DeclaredParams`] of the route it matched
/// ([`ActionNode::declared_params`], the union its `--help` lists): each
/// [`ParamsPartial`] on the route and its ancestors, and each
/// [`DeclaresParams`] component there, ie middleware like [`HelpHandler`], the
/// [`CliServer`]'s `--accept` and the launch knobs (`--entry`, `--stage`, ..)
/// it declares for the argv it serves. A route that forwards what it does not
/// name ([`ParamsPartial::forwarding`]) accepts anything. An unmatched path is
/// refused the same way when it misspells a route ([`RouteTree::closest`]), so
/// `serv` answers `did you mean serve?` rather than not found, and any other
/// is left to the not-found answer, the bigger mistake.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RejectUnknownParams;

impl RejectUnknownParams {
	/// Refuse, as a `400`, `params` for the route `node` matched, naming every
	/// one nothing on its dispatch path declares and pointing at the route's
	/// help. The caller has already established that the url space is
	/// governed ([`RejectUnknownParamsQuery::governs`]).
	pub(crate) async fn check(
		world: &AsyncWorld,
		node: &ActionNode,
		params: MultiMap<SmolStr, SmolStr>,
	) -> Result {
		let route = node.clone();
		let unknown = world
			.with_state::<DeclaredParamsQuery, _>(move |query| {
				route.declared_params(&query).unknown(&params)
			})
			.await;
		let Some(unknown) = unknown else {
			return Ok(());
		};
		Err(HttpError::bad_request(format!(
			"{unknown}\n{}\n",
			node.help_hint()
		))
		.into())
	}

	/// The `400` for an unmatched `path` in a governed url space, naming the
	/// route `near` it most plausibly misspells.
	pub(crate) fn misspelt_route(path: &[SmolStr], near: &str) -> HttpError {
		HttpError::bad_request(format!(
			"unknown route `{}`, did you mean `{near}`?\n`--help` lists the routes\n",
			path.join("/")
		))
	}
}

/// Whether a url space is governed by a [`RejectUnknownParams`], the cheap
/// half of the check, made by a [`Router`] on every request it routes.
#[derive(SystemParam)]
pub(crate) struct RejectUnknownParamsQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	policies: Query<'w, 's, (), With<RejectUnknownParams>>,
	routers: Query<'w, 's, (), With<Router>>,
}

impl RejectUnknownParamsQuery<'_, '_> {
	/// Whether `router` rejects unknown params: a [`RejectUnknownParams`] on it
	/// or on an ancestor below the next enclosing [`Router`].
	pub fn governs(&self, router: Entity) -> bool {
		self.ancestors
			.iter_ancestors_inclusive(router)
			.enumerate()
			.take_while(|(index, entity)| {
				*index == 0 || !self.routers.contains(*entity)
			})
			.any(|(_, entity)| self.policies.contains(entity))
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[derive(Reflect)]
	#[allow(dead_code)]
	struct DeployParams {
		elevated: bool,
		dry_run: bool,
	}

	/// The flag a step of the `audit` sequence reads.
	#[derive(Reflect)]
	#[allow(dead_code)]
	struct AuditParams {
		fix: bool,
	}

	/// Stands in for a command whose params a typo must not silently skip.
	#[action]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn Deploy(_cx: ActionContext<RequestParts>) -> Result<String> {
		Ok("deployed".into())
	}

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// A CLI-shaped root: a server carrying `server`, a router with help, a
	/// `deploy` command taking [`DeployParams`], and an `audit` sequence whose
	/// step declares [`AuditParams`].
	fn spawn_cli(world: &mut World, server: impl Bundle) -> Entity {
		let audit_step =
			Action::<Request, Outcome<Request, Response>>::new_pure(
				|cx: ActionContext<Request>| Outcome::Pass(cx.take()).xok(),
			);
		world
			.spawn((server, children![(
				(Router, HelpHandler::default()),
				children![
					(
						route::exchange("deploy", Deploy),
						ParamsPartial::new::<DeployParams>()
					),
					(PathPartial::new("audit"), ExchangeSequence, children![(
						audit_step,
						ParamsPartial::new::<AuditParams>()
					)]),
				]
			)]))
			.flush()
	}

	/// The router beneath the server root, the entity a server dispatches to.
	fn router(world: &World, root: Entity) -> Entity {
		world.entity(root).get::<Children>().unwrap()[0]
	}

	async fn send(world: &mut World, root: Entity, args: &str) -> Response {
		let router = router(world, root);
		world
			.entity_mut(router)
			.exchange(Request::from_cli_str(args))
			.await
	}

	#[beet_core::test]
	async fn rejects_an_unknown_param_naming_the_closest() {
		let mut world = router_world();
		let root =
			spawn_cli(&mut world, (CliServer::default(), RejectUnknownParams));
		let response = send(&mut world, root, "deploy --elevate").await;
		response.status().xpect_eq(StatusCode::BAD_REQUEST);
		response
			.text()
			.await
			.unwrap()
			.xpect_contains(
				"unknown param `--elevate`, did you mean `--elevated`?",
			)
			.xpect_contains("`deploy --help`");
	}

	/// A misspelt route is the same mistake as a misspelt flag, a different
	/// command run (here none at all), so it is refused naming the route;
	/// a path near nothing is still not found.
	#[beet_core::test]
	async fn rejects_a_misspelt_route_naming_the_closest() {
		let mut world = router_world();
		let root =
			spawn_cli(&mut world, (CliServer::default(), RejectUnknownParams));
		let response = send(&mut world, root, "depoy --elevated").await;
		response.status().xpect_eq(StatusCode::BAD_REQUEST);
		response
			.text()
			.await
			.unwrap()
			.xpect_contains("unknown route `depoy`, did you mean `deploy`?")
			.xpect_contains("`--help`");
		send(&mut world, root, "nonesuch")
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}

	#[beet_core::test]
	async fn accepts_an_unknown_param_without_the_opt_in() {
		let mut world = router_world();
		let root = spawn_cli(&mut world, CliServer::default());
		send(&mut world, root, "deploy --elevate")
			.await
			.unwrap_str()
			.await
			.xpect_contains("deployed");
		send(&mut world, root, "depoy")
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}

	/// The route's own params, a param the router's middleware declares and
	/// the launch knobs the server declares all pass.
	#[beet_core::test]
	async fn accepts_params_declared_along_the_path() {
		let mut world = router_world();
		let root =
			spawn_cli(&mut world, (CliServer::default(), RejectUnknownParams));
		send(
			&mut world,
			root,
			"deploy --elevated --dry_run --entry=site --stage=dev --accept=text/plain",
		)
		.await
		.unwrap_str()
		.await
		.xpect_contains("deployed");
		send(&mut world, root, "deploy --help")
			.await
			.unwrap_str()
			.await
			.xpect_contains("Available routes");
	}

	/// A sequence route runs its steps, so a param a step declares is the
	/// route's: accepted, listed by its help, and named beside a misspelling.
	#[beet_core::test]
	async fn accepts_a_param_a_step_declares() {
		let mut world = router_world();
		let root =
			spawn_cli(&mut world, (CliServer::default(), RejectUnknownParams));
		send(&mut world, root, "audit --fix")
			.await
			.status()
			.xpect_eq(StatusCode::OK);
		send(&mut world, root, "audit --help")
			.await
			.unwrap_str()
			.await
			.xpect_contains("fix");
		send(&mut world, root, "audit --fx")
			.await
			.text()
			.await
			.unwrap()
			.xpect_contains("unknown param `--fx`, did you mean `--fix`?");
	}

	/// A router nested beneath a governed one is a url space of its own: the
	/// site an opted-in CLI serves keeps accepting whatever it is linked with.
	#[beet_core::test]
	async fn a_nested_router_is_not_governed() {
		let mut world = router_world();
		let outer = world.spawn((Router, RejectUnknownParams)).id();
		world.spawn((ChildOf(outer), route::exchange("deploy", Deploy)));
		let mount = world
			.spawn((ChildOf(outer), PathPartial::new("serve")))
			.id();
		let inner = world
			.spawn((ChildOf(mount), Router, children![route::exchange(
				"deploy", Deploy
			)]))
			.flush();
		world
			.entity_mut(outer)
			.exchange(Request::from_cli_str("deploy --utm-source=feed"))
			.await
			.status()
			.xpect_eq(StatusCode::BAD_REQUEST);
		world
			.entity_mut(inner)
			.exchange(Request::from_cli_str("deploy --utm-source=feed"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("deployed");
	}
}
