//! The router's half of a strict url space ([`RejectUnknownParams`]): refusing
//! a param the matched route does not take, and a path misspelling a route.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

impl ActionNode {
	/// Refuse, as a `400`, `params` this route does not take, naming each
	/// and the declared param it most plausibly misspells and pointing at the
	/// route's help. What it takes is its [`ParamsPattern`], so a route taking
	/// [`AnyParams`] refuses nothing. The caller has already
	/// established that the url space is governed
	/// ([`RejectUnknownParamsQuery::governs`]).
	pub(crate) fn reject_unknown_params(
		&self,
		params: &MultiMap<SmolStr, SmolStr>,
	) -> Result {
		match self.params.unknown(params) {
			Some(unknown) => Err(HttpError::bad_request(format!(
				"{unknown}\n{}\n",
				self.help_hint()
			))
			.into()),
			None => Ok(()),
		}
	}
}

impl RouteTree {
	/// The `400` for an unmatched `path` in a strict url space, naming the
	/// route `near` it most plausibly misspells ([`RouteTree::closest`]), so
	/// `serv` answers `did you mean serve?` rather than not found.
	pub(crate) fn misspelt_route(path: &[SmolStr], near: &str) -> HttpError {
		HttpError::bad_request(format!(
			"unknown route `{}`, did you mean `{near}`?\n`--help` lists the routes\n",
			path.join("/")
		))
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

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
	async fn Deploy(
		#[param] elevated: bool,
		#[param] dry_run: bool,
	) -> Result<String> {
		let _ = (elevated, dry_run);
		Ok("deployed".into())
	}

	/// Stands in for a command forwarding every param to a program it runs.
	#[action]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn Forward(any: AnyParams) -> Result<String> {
		let mut keys = any.keys().map(SmolStr::to_string).collect::<Vec<_>>();
		keys.sort();
		Ok(keys.join(","))
	}

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// A CLI-shaped root: `server` over a router with help, a `deploy` command
	/// taking `--elevated --dry-run`, an `audit` sequence whose step declares
	/// [`AuditParams`] and a `forward` command taking any param.
	fn spawn_cli(world: &mut World, server: impl Bundle) -> Entity {
		let audit_step =
			Action::<Request, Outcome<Request, Response>>::new_pure(
				|cx: ActionContext<Request>| Outcome::Pass(cx.take()).xok(),
			);
		world
			.spawn((server, children![(
				(Router, HelpHandler::default()),
				children![
					route::exchange("deploy", Deploy),
					route::exchange("forward", Forward),
					(PathPartial::new("audit"), ExchangeSequence, children![(
						audit_step,
						RouteParams::new::<AuditParams>()
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
		let root = spawn_cli(&mut world, CliServer::default());
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
		let root = spawn_cli(&mut world, CliServer::default());
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

	/// A url space no strict server heads tolerates what it does not read.
	#[beet_core::test]
	async fn accepts_an_unknown_param_where_not_strict() {
		let mut world = router_world();
		let root = spawn_cli(&mut world, ());
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
	/// those the server declares all pass.
	#[beet_core::test]
	async fn accepts_params_declared_along_the_path() {
		let mut world = router_world();
		let root = spawn_cli(&mut world, CliServer::default());
		send(
			&mut world,
			root,
			"deploy --elevated --dry_run --server=cli --accept=text/plain",
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
		let root = spawn_cli(&mut world, CliServer::default());
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
		// a step's param is its route's alone
		send(&mut world, root, "deploy --fix")
			.await
			.status()
			.xpect_eq(StatusCode::BAD_REQUEST);
	}

	/// A route taking any param refuses nothing, even in a strict url space.
	#[beet_core::test]
	async fn any_params_refuses_nothing() {
		let mut world = router_world();
		let root = spawn_cli(&mut world, CliServer::default());
		send(&mut world, root, "forward --nocapture --test-threads=1")
			.await
			.unwrap_str()
			.await
			.xpect_contains("nocapture,test-threads");
	}

	/// A router nested beneath a governed one is a url space of its own: the
	/// site a strict CLI serves keeps accepting whatever it is linked with.
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
