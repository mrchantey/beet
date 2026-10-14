//! The one collector of a route's params.

use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Collects the [`ParamsPattern`] of a route: the union over every reader that
/// runs for it, in three clauses.
///
/// 1. [`MiddlewareParams`] on the route and every ancestor: the middleware and
///    servers a request reaching it passes through.
/// 2. [`RouteParams`] on the route: its own action and exchange adapter.
/// 3. [`RouteParams`] on its steps: every descendant that is not itself a route
///    or a url space root (no [`PathPartial`]), and not beneath one, ie the
///    `<ZoneAudit/>` an `<Route path="audit" {ExchangeSequence}>` runs, and
///    every member of a [`Group`] a [`RunGroup`] there runs, authored wherever
///    it is, ie the `<TofuDestroy/>` a stack's `destroy` route runs by
///    reference.
///
/// Nothing is keyed on a route's output type: a scene route lists `--root`
/// because its exchange adapter declared [`RenderParams`] like any reader.
///
/// No reader may take a launch only knob (`--entry`, `--features`, ..): the
/// launch strips those from the process request ([`BootstrapConfig::strip`]),
/// so a command declaring one would never receive it.
///
/// ```mermaid
/// flowchart TB
///     S["CliServer<br/>MiddlewareParams: accept, server"] --> R["Router<br/>MiddlewareParams: help, navigate, color-scheme"]
///     R --> G["stack, a PathPartial group"]
///     G --> D["deploy route<br/>RouteParams: elevated, dry-run, roll (DeployGate), force (ExchangeGroup)"]
///     G --> A["audit route, ExchangeSequence"]
///     A --> Z["ZoneAudit step<br/>RouteParams: fix"]
/// ```
///
/// `audit` takes `--fix` and the inherited `--help --navigate --color-scheme
/// --accept --server`; `deploy` takes `--elevated --dry-run --roll --force`
/// plus the same inherited set. Neither leaks to the other.
#[derive(SystemParam)]
pub struct ParamsQuery<'w, 's> {
	ancestors: Query<'w, 's, &'static ChildOf>,
	children: Query<'w, 's, &'static Children>,
	members: Query<'w, 's, &'static Members>,
	groups: Query<'w, 's, (Entity, &'static RunGroup<Request, Response>)>,
	paths: Query<'w, 's, (), With<PathPartial>>,
	route_params: Query<'w, 's, &'static RouteParams>,
	middleware_params: Query<'w, 's, &'static MiddlewareParams>,
}

impl ParamsQuery<'_, '_> {
	/// The pattern of the route at `route`: its own declarations, then its
	/// steps', then the middleware on it and each ancestor, nearest first.
	///
	/// ## Errors
	///
	/// Errors when two declarers name one key with a different type or arity,
	/// see [`ParamsPattern::from_declarations`], or a declarer takes a launch
	/// only knob.
	pub fn of(&self, route: Entity) -> Result<ParamsPattern> {
		let own = core::iter::once(route).chain(self.steps(route)).filter_map(
			|entity| {
				self.route_params
					.get(entity)
					.ok()
					.map(|list| (entity, Reach::Route, list.as_slice()))
			},
		);
		let middleware =
			self.ancestors.iter_ancestors_inclusive(route).filter_map(
				|entity| {
					self.middleware_params.get(entity).ok().map(|list| {
						(entity, Reach::Middleware, list.as_slice())
					})
				},
			);
		let pattern = ParamsPattern::from_declarations(
			own.chain(middleware).flat_map(|(entity, reach, list)| {
				list.iter()
					.map(move |declaration| (entity, reach, declaration))
			}),
		)?;
		Self::reject_launch_knobs(&pattern)?;
		pattern.xok()
	}

	/// Refuse a param named after a launch only knob, which the process
	/// request never carries.
	fn reject_launch_knobs(pattern: &ParamsPattern) -> Result {
		let Some(item) = pattern.items().iter().find(|item| {
			let key = item.meta.name();
			!BootstrapConfig::REQUEST_RANK.contains(&key)
				&& BootstrapConfig::arg_names().any(|knob| knob == key)
		}) else {
			return Ok(());
		};
		bevybail!(
			"`{}` takes `--{}`, a launch knob the process request never \
			carries: name the param otherwise",
			item.declarers[0].reader,
			item.meta.name()
		)
	}

	/// Every step of `route`: what runs beneath it, down to, and excluding,
	/// the next [`PathPartial`] on each branch.
	fn steps(&self, route: Entity) -> Vec<Entity> {
		let mut steps = Vec::new();
		let mut visited = HashSet::<Entity>::default();
		let mut stack = self.runs(route);
		while let Some(entity) = stack.pop() {
			if self.paths.contains(entity) || !visited.insert(entity) {
				continue;
			}
			steps.push(entity);
			stack.extend(self.runs(entity));
		}
		steps
	}

	/// What runs beneath `entity`: its children, the members of a group it is,
	/// and the members of the group a [`RunGroup`] on it runs.
	fn runs(&self, entity: Entity) -> Vec<Entity> {
		let run = self
			.groups
			.get(entity)
			.ok()
			.map(|(host, run)| run.group.unwrap_or(host));
		core::iter::once(entity)
			.chain(run)
			.filter_map(|group| self.members.get(group).ok())
			.flat_map(|members| members.iter())
			.chain(self.children.get(entity).into_iter().flatten().copied())
			.collect()
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
	struct Fix {
		fix: bool,
	}

	#[derive(Reflect)]
	#[allow(dead_code)]
	struct Elevated {
		elevated: bool,
	}

	#[derive(Reflect)]
	#[allow(dead_code)]
	struct Verbose {
		verbose: bool,
	}

	#[derive(Reflect)]
	#[allow(dead_code)]
	struct Nested {
		nested: bool,
	}

	fn names(world: &mut World, route: Entity) -> Vec<String> {
		world.with_state::<ParamsQuery, _>(|query| {
			query
				.of(route)
				.unwrap()
				.params()
				.map(|meta| meta.name().to_string())
				.collect()
		})
	}

	/// The three clauses: middleware on the route and above, the route's own,
	/// its steps'; never a sibling's, and never a nested route's.
	#[beet_core::test]
	fn collects_the_three_clauses() {
		let mut world = World::new();
		let root = world.spawn(MiddlewareParams::new::<Verbose>()).id();
		let deploy = world
			.spawn((
				ChildOf(root),
				PathPartial::new("deploy"),
				RouteParams::new::<Elevated>(),
			))
			.id();
		let audit = world
			.spawn((ChildOf(root), PathPartial::new("audit"), children![(
				RouteParams::new::<Fix>(),
				children![(
					PathPartial::new("nested"),
					RouteParams::new::<Nested>()
				)]
			)]))
			.id();
		names(&mut world, deploy).xpect_eq(vec!["elevated", "verbose"]);
		names(&mut world, audit).xpect_eq(vec!["fix", "verbose"]);
	}

	/// A param named after a launch only knob never arrives, so it is refused.
	#[beet_core::test]
	fn launch_knobs_are_refused() {
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct Features {
			features: Option<String>,
		}
		let mut world = World::new();
		let route = world.spawn(RouteParams::new::<Features>()).id();
		world
			.with_state::<ParamsQuery, _>(|query| query.of(route))
			.unwrap_err()
			.to_string()
			.xpect_contains("`Features` takes `--features`, a launch knob");
	}

	/// A route running a group by reference runs its members, wherever they
	/// are authored, so their params are the route's.
	#[beet_core::test]
	fn collects_a_referenced_groups_members() {
		let mut world = World::new();
		let group = world.spawn(Group).id();
		world.spawn((MemberOf(group), RouteParams::new::<Fix>()));
		let route = world
			.spawn((
				PathPartial::new("destroy"),
				RunGroup::<Request, Response>::reversed(group),
			))
			.id();
		names(&mut world, route).xpect_eq(vec!["fix"]);
	}

	/// Two declarers of one key with different types are an error.
	#[beet_core::test]
	fn conflicts_are_errors() {
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct PortNumber {
			port: u16,
		}
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct PortName {
			port: String,
		}
		let mut world = World::new();
		let root = world.spawn(MiddlewareParams::new::<PortNumber>()).id();
		let route = world
			.spawn((ChildOf(root), RouteParams::new::<PortName>()))
			.id();
		world
			.with_state::<ParamsQuery, _>(|query| query.of(route))
			.unwrap_err()
			.to_string()
			.xpect_contains("conflicting declarations of `--port`");
	}
}
