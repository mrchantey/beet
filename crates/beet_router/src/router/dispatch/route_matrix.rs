//! One verb fanned across sibling namespaces: the single-command bring-up and
//! teardown a multi-stack entry advertises.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Marker for a route that fans each of its `routes` out across every namespace
/// in `prefixes`:
///
/// ```bsx
/// <Route path="all" {RouteMatrix{prefixes:["site","mail","social"], routes:["validate","plan","deploy","destroy"]}}/>
/// ```
///
/// So `all/deploy` calls `site/deploy`, then `mail/deploy`, then
/// `social/deploy`, in the listed order, sequentially, aborting at the first
/// failure.
///
/// Aggregation is EXPLICIT: both lists are hand-written and nothing is
/// discovered, so the fan-out reads off the entry rather than off whatever
/// happened to be spawned. The cost of that is a typo being possible, which is
/// why an unresolvable target is a hard error rather than a skip.
///
/// The lifecycle splits in two, and the split is the whole design:
///
/// - **On insert** one child ROUTE ENTITY is spawned per entry in `routes`. The
///   paths come from the literal list, so nothing is looked up and `all/deploy`
///   is in `--help` immediately. One entity per verb rather than one entity
///   dispatching several, because a route's path is a component on its entity
///   and a single entity cannot carry four of them.
/// - **On [`Ready`]** each child resolves its `<prefix>/<verb>` targets through
///   the ancestor [`RouteTree`] ([`RouteMatrixStep`]). This cannot happen at
///   insert time: sibling routes are still registering then, and a target
///   arriving through a `<Template src>` include has not built yet. `Ready` is
///   precisely the point at which every level of include has settled, so it is
///   both correct and the earliest correct moment.
///
/// The [`Ready`] sweep runs deepest-first with the loaded root last, so a
/// child's resolution always lands before the root's `CallOnReady` dispatches
/// the requested command: a mistyped target fails the load rather than letting
/// a partially-resolved matrix run half a fan-out.
///
/// One listed order serves every verb. Reverse-order teardown is not offered:
/// it only matters where stacks reference each other's resources, and a matrix
/// whose namespaces do would want that as a declared property, not an implicit
/// one.
///
/// A guarded resource refusing to be destroyed (a bucket without
/// `force_destroy`, a protected volume) fails `all/destroy` at that namespace.
/// That is the guard working; the single-command teardown is honest about a
/// stack that has not disarmed its own.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::component_hook(RouteMatrix::spawn_verbs))]
pub struct RouteMatrix {
	/// The sibling namespaces each verb fans out to, in invocation order, ie
	/// `["site","mail","social"]`.
	pub prefixes: Vec<SmolStr>,
	/// The verbs to mount under this route, each becoming a child route of the
	/// same name, ie `["validate","plan","deploy","destroy"]`.
	pub routes: Vec<SmolStr>,
}

impl RouteMatrix {
	/// The matrix over `prefixes` x `routes`, the code counterpart of the markup
	/// spread.
	pub fn new(
		prefixes: impl IntoIterator<Item = impl Into<SmolStr>>,
		routes: impl IntoIterator<Item = impl Into<SmolStr>>,
	) -> Self {
		Self {
			prefixes: prefixes.into_iter().map(Into::into).collect(),
			routes: routes.into_iter().map(Into::into).collect(),
		}
	}

	/// The insert half: one child route per declared verb, each carrying the
	/// prefix list it will resolve against on [`Ready`].
	///
	/// `on_add` rather than `on_insert`, so a reload that re-inserts the same
	/// matrix does not spawn a second set of verbs beside the first (which the
	/// route tree would reject as duplicate paths).
	fn spawn_verbs(matrix: &Self) -> impl FnOnce(&mut EntityCommands) + use<> {
		let prefixes = matrix.prefixes.clone();
		let routes = matrix.routes.clone();
		move |entity| {
			entity.with_children(|parent| {
				for verb in routes {
					parent.spawn((
						PathPartial::new(verb.as_str()),
						RouteMatrixStep {
							verb,
							prefixes: prefixes.clone(),
							targets: Vec::new(),
						},
						RouteMatrixDispatch,
					));
				}
			});
		}
	}
}

/// One verb of a [`RouteMatrix`], on its own route entity: the namespaces it
/// fans out to, and the targets it resolved them to on [`Ready`].
///
/// The resolution is cached here rather than repeated per call, so a request
/// dispatches straight into the targets and a bad prefix is caught once, at
/// load, instead of on the first invocation.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::observe(RouteMatrixStep::resolve_on_ready))]
pub struct RouteMatrixStep {
	/// The verb this route serves, ie `deploy`.
	pub verb: SmolStr,
	/// The namespaces to call it in, in invocation order.
	pub prefixes: Vec<SmolStr>,
	/// The resolved targets, filled on [`Ready`] and empty before it.
	pub targets: Vec<RouteMatrixTarget>,
}

/// One resolved `<prefix>/<verb>`: the path to address it by and the route
/// entity serving it.
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub struct RouteMatrixTarget {
	/// The path segments, ie `["mail","deploy"]`.
	pub path: Vec<SmolStr>,
	/// The route entity the tree resolved that path to.
	pub entity: Entity,
}

// `Entity` has no `Default`, so the placeholder stands for "not yet resolved";
// every live target is written by the `Ready` pass.
impl Default for RouteMatrixTarget {
	fn default() -> Self {
		Self {
			path: Vec::new(),
			entity: Entity::PLACEHOLDER,
		}
	}
}

impl RouteMatrixStep {
	/// Observer: resolve every `<prefix>/<verb>` through the ancestor
	/// [`RouteTree`] once the load settles.
	///
	/// A prefix that names no route is a HARD error: the list is hand-written,
	/// so a typo must fail loudly rather than quietly shrinking the fan-out. The
	/// failure rides [`TemplateError`] on the swept root, which is the same
	/// channel a failed build uses and which `CallOnReady` (firing last, on that
	/// root) turns into a nonzero exit — so nothing runs against a
	/// half-resolved matrix.
	fn resolve_on_ready(
		ev: On<Ready>,
		mut steps: Query<&mut RouteMatrixStep>,
		trees: AncestorQuery<&RouteTree>,
		mut commands: Commands,
	) {
		let Ok(step) = steps.get(ev.entity) else {
			return;
		};
		let (verb, prefixes) = (step.verb.clone(), step.prefixes.clone());
		let resolved = trees
			.get(ev.entity)
			.map_err(|_| {
				bevyhow!(
					"`RouteMatrix` verb `{verb}` sits in no url space: it needs \
					 a `Router` ancestor to resolve its targets through"
				)
			})
			.and_then(|tree| Self::resolve(tree, &verb, &prefixes));
		match resolved {
			Ok(targets) => {
				if let Ok(mut step) = steps.get_mut(ev.entity) {
					step.targets = targets;
				}
			}
			Err(err) => {
				error!("{err}");
				commands
					.entity(ev.trigger().root())
					.insert(TemplateError::new(CloneError::new(err)));
			}
		}
	}

	/// Every `<prefix>/<verb>` looked up in `tree`, in the declared order.
	fn resolve(
		tree: &RouteTree,
		verb: &SmolStr,
		prefixes: &[SmolStr],
	) -> Result<Vec<RouteMatrixTarget>> {
		prefixes
			.iter()
			.map(|prefix| {
				let path = vec![prefix.clone(), verb.clone()];
				let node = tree.find(&path).ok_or_else(|| {
					bevyhow!(
						"`RouteMatrix` names `{prefix}/{verb}`, which this \
						 router serves no route for. Either the namespace is \
						 misspelled or it declares no `{verb}`."
					)
				})?;
				RouteMatrixTarget {
					path,
					entity: node.entity,
				}
				.xok()
			})
			.collect()
	}
}

/// Run this verb across every namespace the matrix names, in order.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn RouteMatrixDispatch(
	cx: ActionContext<Request>,
) -> Result<Response> {
	let step = cx.caller.get(|step: &RouteMatrixStep| step.clone()).await?;
	if step.targets.is_empty() {
		bevybail!(
			"`{}` resolved no targets: the matrix's `Ready` pass never ran, so \
			 nothing was dispatched",
			step.verb
		);
	}
	// the incoming parts thread through unchanged apart from the path, so a
	// `--stage=prod` or a `--force` reaches every namespace exactly as it would
	// a direct call. The body does not: a cli verb carries none, and one body
	// cannot be read by several targets.
	let parts = cx.input.parts().clone();
	let world = cx.caller.world();
	let mut output = String::new();
	for target in step.targets.iter() {
		let path = target.path.join("/");
		let mut parts = parts.clone();
		parts.url_mut().set_path(target.path.clone());
		info!("{} -> {path}", step.verb);
		let response = world
			.entity(target.entity)
			.exchange(Request::from_parts(parts, Body::default()))
			.await;
		let status = response.status();
		let body = response.text().await.unwrap_or_default();
		output.push_str(&format!("== {path} ==\n{body}\n"));
		// fail fast: a namespace that failed leaves the rest unattempted, since
		// continuing would report a success the run did not have.
		if !status.is_ok() {
			return Response::from_status(status).with_body(output).xok();
		}
	}
	Response::ok().with_body(output).xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// Sweep `Ready` over `root`, the load boundary the matrix resolves its
	/// targets on. A hand-spawned test tree never rides a template build, so it
	/// fires the sweep itself.
	fn settle(world: &mut World, root: Entity) {
		world
			.entity_mut(root)
			.trigger_target(|entity| Ready { entity });
		world.flush();
	}

	/// A route recording its own path into `log` when called, the step every
	/// order assertion is built from.
	fn recorded(
		path: &str,
		log: Store<Vec<String>>,
	) -> (PathPartial, Action<Request, Response>) {
		let name = path.to_string();
		(
			PathPartial::new(path),
			Action::new_async(move |_cx: ActionContext<Request>| {
				let (log, name) = (log.clone(), name.clone());
				async move {
					let mut all = log.get();
					all.push(name.clone());
					log.set(all);
					Response::ok().with_body(name).xok()
				}
			}),
		)
	}

	/// A route that always fails, for the abort case.
	fn failing(path: &str) -> (PathPartial, Action<Request, Response>) {
		(
			PathPartial::new(path),
			Action::new_async(async |_: ActionContext<Request>| {
				Response::from_status(StatusCode::IM_A_TEAPOT).xok()
			}),
		)
	}

	/// The matrix mounts one child route per declared verb at insert, so
	/// `all/deploy` is dispatchable (and in `--help`) without anything being
	/// discovered.
	#[beet_core::test]
	fn verbs_are_routes_immediately() {
		let mut world = router_world();
		let root = world
			.spawn((Router::with_defaults(), children![(
				PathPartial::new("all"),
				RouteMatrix::new(["site"], ["plan", "deploy"]),
			)]))
			.flush();
		let tree = RouteTree::of(&world, root).unwrap();
		tree.find(&["all", "plan"]).xpect_some();
		tree.find(&["all", "deploy"]).xpect_some();
	}

	/// Two sub-routers sharing verb names are called in the LISTED order, not
	/// the spawn order, which is the whole promise of an explicit matrix.
	#[beet_core::test]
	async fn dispatches_in_listed_order() {
		let log = Store::new(Vec::<String>::new());
		let mut world = router_world();
		let root = world
			.spawn((Router::with_defaults(), children![
				(PathPartial::new("site"), children![recorded(
					"deploy",
					log.clone()
				)]),
				(PathPartial::new("mail"), children![recorded(
					"deploy",
					log.clone()
				)]),
				(
					PathPartial::new("all"),
					// mail before site, the reverse of the spawn order above
					RouteMatrix::new(["mail", "site"], ["deploy"]),
				),
			]))
			.flush();
		settle(&mut world, root);
		world
			.entity_mut(root)
			.exchange(Request::get("all/deploy"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
		log.get().xpect_eq(vec!["deploy".to_string(); 2]);
		// the response carries each namespace's own output, in the same order
		let body = world
			.entity_mut(root)
			.exchange_str(Request::get("all/deploy"))
			.await;
		(body.find("mail/deploy") < body.find("site/deploy")).xpect_true();
	}

	/// A failing step halts the chain: the namespaces after it are never
	/// called, and the failing status is what the matrix answers.
	#[beet_core::test]
	async fn a_failing_step_aborts_the_sequence() {
		let log = Store::new(Vec::<String>::new());
		let mut world = router_world();
		let root = world
			.spawn((Router::with_defaults(), children![
				(PathPartial::new("site"), children![recorded(
					"deploy",
					log.clone()
				)]),
				(PathPartial::new("mail"), children![failing("deploy")]),
				(PathPartial::new("social"), children![recorded(
					"deploy",
					log.clone()
				)]),
				(
					PathPartial::new("all"),
					RouteMatrix::new(["site", "mail", "social"], ["deploy"]),
				),
			]))
			.flush();
		settle(&mut world, root);
		world
			.entity_mut(root)
			.exchange(Request::get("all/deploy"))
			.await
			.status()
			.xpect_eq(StatusCode::IM_A_TEAPOT);
		// site ran, social never did
		log.get().len().xpect_eq(1);
	}

	/// A prefix naming no route fails the LOAD with the offending path named,
	/// rather than quietly fanning out to whatever did resolve.
	#[beet_core::test]
	fn an_unknown_target_fails_the_load() {
		let mut world = router_world();
		let root = world
			.spawn(children![(Router::with_defaults(), children![
				(PathPartial::new("site"), children![(
					PathPartial::new("deploy"),
					Action::<Request, Response>::new_async(
						async |_: ActionContext<Request>| Response::ok().xok()
					),
				)]),
				(
					PathPartial::new("all"),
					RouteMatrix::new(["site", "typo"], ["deploy"]),
				),
			])])
			.flush();
		settle(&mut world, root);
		// the failure rides `TemplateError` on the swept root, the channel
		// `CallOnReady` turns into a nonzero exit
		world
			.entity(root)
			.get::<TemplateError>()
			.unwrap()
			.error
			.to_string()
			.xpect_contains("typo/deploy");
	}

	/// Targets that arrive through a `<Template src>` include resolve fine: the
	/// include is a structural pending dependency, so `Ready` is deferred until
	/// it has built. The regression this pins is a slide back to insert-time
	/// resolution, which would see no `site` route at all.
	#[cfg(feature = "template_serde")]
	#[beet_core::test]
	async fn targets_from_an_include_resolve() {
		let mut world = router_world();
		register_template_include(&mut world);

		let store = BlobStore::temp();
		store
			.insert(
				&SmolPath::from("site.bsx"),
				r#"<Route path="site"><Route path="deploy" {ExchangeSequence}/></Route>"#,
			)
			.await
			.unwrap();

		let root = BsxTemplate::parse_entry(
			&world,
			r#"<Router><Template src="site.bsx"/><Route path="all" {RouteMatrix{prefixes:["site"], routes:["deploy"]}}/></Router>"#,
		)
		.unwrap()
		.spawn(&mut world)
		.unwrap();
		world.entity_mut(root).insert(store);
		AsyncRunner::settle_async_tasks(&mut world).await;

		// the include built, so the matrix resolved through it rather than
		// failing the load
		world.entity(root).get::<TemplateError>().xpect_none();
		world
			.query::<&RouteMatrixStep>()
			.single(&world)
			.unwrap()
			.targets
			.len()
			.xpect_eq(1);
	}
}
