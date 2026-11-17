//! [`ThreadRoute`], a route answering each request with a thread built for it.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// A route answering each request with a thread built for it from the named
/// template, each thread one thread for its whole life:
///
/// ```bsx
/// <Route path="review/:document" {ThreadRoute{template:"ReviewThread"}}/>
/// ```
///
/// On every request it:
///
/// 1. builds a fresh thread from `template`, a name resolved as a [`Layout`]
///    resolves its own ([`BsxTemplate::named`]), with the request on the
///    [`RequestContextStack`] for the build, so a seed reads a path param
///    (`review {document}`) from a `#[template(system)]` taking
///    `Res<RequestContextStack>`;
/// 2. settles the build, every [`SeedSource`] read, and attaches the thread
///    beneath the route, where it resolves the ancestor [`BlobStore`] and the
///    host url space as if authored there. It is built detached first, so the
///    host [`RouteTree`] never rebuilds for it;
/// 3. enters it ([`Thread::enter`]): a second request seeding the same thread
///    adopts it by seed hash, another seed is a thread of its own, and
///    `--new` discards the stored one;
/// 4. runs the template's behavior to its [`Outcome`];
/// 5. waits for every completed post to reach its [`ThreadStore`]
///    ([`ThreadStore::flush`]), then despawns the thread;
/// 6. answers the last completed agent text as the body on a pass, and a `500`
///    naming the failure on a fail.
///
/// A failed build, a seed that could not be read or a behavior that errors
/// fail the request the same way, the thread despawned all the same.
#[action]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn ThreadRoute(
	/// The template building the thread: a `.bsx` document in the
	/// [`BsxTemplateRegistry`] or a rust `#[template]`, by name.
	#[field]
	template: SmolStr,
	/// Discard the stored thread this request seeds and start it afresh.
	#[param]
	new: bool,
	cx: ActionContext<Request>,
) -> Result<Response> {
	let route = cx.caller.clone();
	let world = route.world().clone();
	let root =
		ThreadRoute::build(&route, template.clone(), cx.input.parts().clone())
			.await?;
	let answer = ThreadRoute::run(&world.entity(root), &template, new).await;
	// torn down however the run ended, the response answered after
	world
		.with(move |world: &mut World| ThreadRoute::despawn(world, root))
		.await;
	answer
}

impl ThreadRoute {
	/// Build `template` for the request `parts`, settle it and attach it
	/// beneath `route`, answering the build root, the container holding the
	/// template's own root.
	async fn build(
		route: &AsyncEntity,
		template: SmolStr,
		parts: RequestParts,
	) -> Result<Entity> {
		let route_id = route.id();
		let root = route
			.world()
			.with(move |world: &mut World| -> Result<Entity> {
				let template = BsxTemplate::named(world, &template)?;
				let router = world
					.with_state::<AncestorQuery<&RouteTree>, _>(|trees| {
						trees.get_entity(route_id)
					})
					.unwrap_or(route_id);
				let root = world.spawn_empty().id();
				RequestContextStack::scoped(
					world,
					RequestContext::new(parts, root, route_id, router),
					|world| {
						world
							.entity_mut(root)
							.insert_template(template)
							.map(|_| ())
					},
				)?;
				// the build's route tree wakes run while the thread is still
				// detached, so they reach its own url spaces and never the
				// host's; attached after, it resolves its ancestors as if
				// authored beneath the route, before any seed is read
				world.flush();
				world.entity_mut(root).insert(ChildOf(route_id));
				root.xok()
			})
			.await?;
		TemplatePending::settle_root(&route.world().entity(root)).await;
		let error = route
			.world()
			.with(move |world: &mut World| {
				world
					.get::<TemplateError>(root)
					.map(|error| error.error.clone())
			})
			.await;
		match error {
			Some(error) => {
				route
					.world()
					.with(move |world: &mut World| Self::despawn(world, root))
					.await;
				Err(error.into())
			}
			None => root.xok(),
		}
	}

	/// Enter the threads under the build `root`, run the template's behavior,
	/// flush every thread and answer.
	async fn run(
		root: &AsyncEntity,
		template: &str,
		new: bool,
	) -> Result<Response> {
		RunThread::enter_all(root, new).await?;
		let (behavior, threads) = root
			.with_state::<(Query<&Children>, Query<(), With<Thread>>), _>(
				|root, (children, threads)| {
					(
						children.get(root).ok().and_then(|children| {
							(children.len() == 1).then(|| children[0])
						}),
						children
							.iter_descendants(root)
							.filter(|entity| threads.contains(*entity))
							.collect::<Vec<_>>(),
					)
				},
			)
			.await?;
		let behavior = behavior.ok_or_else(|| {
			bevyhow!("the template `{template}` must build exactly one root")
		})?;
		let thread = *threads.first().ok_or_else(|| {
			bevyhow!("the template `{template}` builds no `<Thread>`")
		})?;
		let outcome = root
			.world()
			.entity(behavior)
			.call::<(), Outcome>(())
			.await?;
		for thread in &threads {
			ThreadStore::flush(&root.world().entity(*thread)).await?;
		}
		let last_text = root
			.world()
			.with(move |world: &mut World| Self::last_agent_text(world, thread))
			.await;
		match (outcome, last_text) {
			(Outcome::Pass(_), Some(text)) => {
				Response::ok_body(text, MediaType::Text)
			}
			(Outcome::Pass(_), None) => Response::ok(),
			(Outcome::Fail(_), text) => Response::from_status_body(
				StatusCode::INTERNAL_SERVER_ERROR,
				format!(
					"the thread `{template}` failed{}",
					text.map(|text| format!(", its last word: {text}"))
						.unwrap_or_default()
				),
				MediaType::Text,
			),
		}
		.xok()
	}

	/// Tear down the build `root`: detached from the route first, then every
	/// entity leaves first, so each route removal records the url space it
	/// leaves while its ancestry is whole and the host [`RouteTree`] never
	/// rebuilds for a thread that was never part of it.
	fn despawn(world: &mut World, root: Entity) {
		let Ok(mut root_mut) = world.get_entity_mut(root) else {
			return;
		};
		root_mut.remove::<ChildOf>();
		let entities = world.with_state::<Query<&Children>, _>(|children| {
			children
				.iter_descendants_inclusive(root)
				.collect::<Vec<_>>()
		});
		for entity in entities.into_iter().rev() {
			if let Ok(entity) = world.get_entity_mut(entity) {
				entity.despawn();
			}
		}
	}

	/// The last completed text an agent posted in `thread`.
	fn last_agent_text(world: &World, thread: Entity) -> Option<String> {
		let window = world.get::<ThreadWindow>(thread)?;
		window
			.post_views()
			.filter(|view| {
				view.actor.kind() == ActorKind::Agent
					&& !view.post.in_progress()
			})
			.filter_map(|view| match view.post.as_agent_post() {
				AgentPost::Text(text) => Some(text.to_string()),
				_ => None,
			})
			.last()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_router::prelude::*;

	/// The seed of a review thread, read off the request's
	/// `document` path param while the thread builds.
	#[template(system)]
	fn ReviewSeed(stack: Res<RequestContextStack>) -> impl Bundle {
		let document = stack
			.current()
			.parts()
			.get_param("document")
			.unwrap_or_default()
			.to_string();
		Post::spawn(format!("review {document}"))
	}

	/// The thread a review builds per request: pinned actors, the reviewed
	/// document in its seed, a reading agent with tools of its own that reads once then
	/// answers, its records kept under `threads`.
	const REVIEW_THREAD: &str = r#"
<RepeatWhileFunctionCallOutput>
	<Thread {(Sequence, MountThreadStore{path:"threads"})}>
		<CreateActor name="User" kind="User" id=1><ReviewSeed/></CreateActor>
		<CreateActor name="Agent" kind="Agent" id=2 {MockPostStreamer{script:[Call{tool:"read-blob", arguments:{path:"notes.md"}}]}}>
			<Router><StoreToolset read_only=true/></Router>
		</CreateActor>
	</Thread>
</RepeatWhileFunctionCallOutput>
"#;

	/// A host url space holding the review route over an in-memory store.
	fn host() -> (App, Entity, BlobStore) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_type::<(ThreadPlugin, ReviewSeed)>();
		app.world_mut()
			.get_resource_or_init::<BsxTemplateRegistry>()
			.insert_source("ReviewThread", REVIEW_THREAD)
			.unwrap();
		let store = BlobStore::temp();
		let router = BsxTemplate::parse_entry(
			app.world(),
			r#"<Router><Route path="review/:document" {ThreadRoute{template:"ReviewThread"}}/></Router>"#,
		)
		.unwrap()
		.spawn(app.world_mut())
		.unwrap();
		app.world_mut().entity_mut(router).insert(store.clone());
		(app, router, store)
	}

	/// Request `request` of the host, answering its status and body.
	async fn review(
		app: &mut App,
		router: Entity,
		request: Request,
	) -> (StatusCode, String) {
		let response =
			app.world_mut().entity_mut(router).exchange(request).await;
		(response.status(), response.text().await.unwrap())
	}

	/// Every stored thread's text posts, the tool calls aside, in seed order.
	async fn stored(store: &BlobStore) -> Vec<Vec<String>> {
		let records =
			BlobThreadStore::new(store.with_subdir(RelPath::new("threads")));
		let mut threads = Vec::new();
		for thread in records.threads().await.unwrap() {
			threads.push(
				records
					.thread_posts(thread.id(), None)
					.await
					.unwrap()
					.iter()
					.filter(|post| {
						matches!(post.as_agent_post(), AgentPost::Text(_))
					})
					.map(|post| post.to_string())
					.collect::<Vec<_>>(),
			);
		}
		threads.sort();
		threads
	}

	/// The thread answers with its agent's last text, and is gone once it has,
	/// its posts in the store and the host's route tree untouched.
	#[beet_core::test]
	async fn answers_then_despawns_its_thread() {
		let (mut app, router, store) = host();
		let tree = app
			.world()
			.entity(router)
			.get_ref::<RouteTree>()
			.unwrap()
			.last_changed();
		review(&mut app, router, Request::get("review/product"))
			.await
			.xpect_eq((StatusCode::OK, "you said: review product".to_string()));
		app.world_mut()
			.query_filtered::<(), With<Thread>>()
			.iter(app.world())
			.count()
			.xpect_eq(0);
		stored(&store).await.xpect_eq(vec![vec![
			"review product".to_string(),
			"you said: review product".to_string(),
		]]);
		app.world()
			.entity(router)
			.get_ref::<RouteTree>()
			.unwrap()
			.last_changed()
			.xpect_eq(tree);
	}

	/// A second request for one document resumes its stored thread, and a
	/// request for another, whose seed reads its own path param, is a thread
	/// of its own.
	#[beet_core::test]
	async fn each_seed_is_one_thread() {
		let (mut app, router, store) = host();
		for path in ["review/product", "review/product", "review/market"] {
			review(&mut app, router, Request::get(path))
				.await
				.0
				.xpect_eq(StatusCode::OK);
		}
		stored(&store).await.xpect_eq(vec![
			vec![
				"review market".to_string(),
				"you said: review market".to_string(),
			],
			vec![
				"review product".to_string(),
				"you said: review product".to_string(),
				"you said: review product".to_string(),
			],
		]);
	}

	/// `--new` discards the stored thread its request seeds and starts it
	/// afresh, leaving every other.
	#[beet_core::test]
	async fn new_discards_the_stored_thread() {
		let (mut app, router, store) = host();
		for request in [
			Request::get("review/product"),
			Request::get("review/market"),
			Request::get("review/product").with_param("new", "true"),
		] {
			review(&mut app, router, request)
				.await
				.0
				.xpect_eq(StatusCode::OK);
		}
		stored(&store).await.xpect_eq(vec![
			vec![
				"review market".to_string(),
				"you said: review market".to_string(),
			],
			vec![
				"review product".to_string(),
				"you said: review product".to_string(),
			],
		]);
	}
}
