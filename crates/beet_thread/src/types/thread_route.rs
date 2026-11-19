//! [`ThreadRoute`], a route answering each request with a thread built for it.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// A route answering each request with a session of a thread built for it
/// from the named template, each thread entity one session of one thread:
///
/// ```bsx
/// <Route path="review/:document" {ThreadRoute{template:"ReviewThread", metadata:{document:"{document}"}}}/>
/// ```
///
/// On every request it:
///
/// 1. builds a fresh thread from `template`, a name resolved as a [`Layout`]
///    resolves its own ([`BsxTemplate::named`]), with the request on the
///    [`RequestContextStack`] for the build, so a seed reads a path param,
///    ie `<RequestPost text="review {document}"/>`;
/// 2. settles the build, every [`SeedSource`] read, and attaches the thread
///    beneath the route, where it resolves the ancestor [`BlobStore`] and the
///    host url space as if authored there. It is built detached first, so the
///    host [`RouteTree`] never rebuilds for it;
/// 3. enters it ([`Thread::enter`]), continuing the newest stored thread
///    whose metadata holds every entry of `metadata`, each `{param}` filled
///    from the request, plus `template`; a route declaring no metadata starts
///    a thread every request. `--thread` continues a stored thread by id,
///    `--new` starts one stamped the same, and `--spawned-by` names the post
///    whose action asked for the session ([`ThreadEntryParams`]);
/// 4. runs the template's behavior to its [`Outcome`], then closes the
///    session ([`SessionClosed`]), an error closing it as a fail;
/// 5. waits for every completed post to reach its [`ThreadStore`]
///    ([`ThreadStore::flush`]), then despawns the thread;
/// 6. answers the last completed text of the thread's acting actors as the
///    body on a pass, and a `500` naming the failure on a fail.
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
	/// The entries a thread this route creates is stamped with, each string's
	/// `{param}` filled from the request, and that a request continues a
	/// stored thread by. Empty starts a thread every request.
	#[field]
	metadata: Map,
	entry: Params<ThreadEntryParams>,
	cx: ActionContext<Request>,
) -> Result<Response> {
	let route = cx.caller.clone();
	let world = route.world().clone();
	let entry = ThreadRoute::entry(
		&template,
		&metadata,
		cx.input.parts(),
		ThreadEntryParams::clone(&entry),
	)?;
	let root =
		ThreadRoute::build(&route, template.clone(), cx.input.parts().clone())
			.await?;
	let answer = ThreadRoute::run(&world.entity(root), &template, entry).await;
	// torn down however the run ended, the response answered after
	world
		.with(move |world: &mut World| ThreadRoute::despawn(world, root))
		.await;
	answer
}

impl ThreadRoute {
	/// The entry a request makes: `metadata` filled from `parts` and stamped
	/// with the template, continuing by it when the route declares any.
	fn entry(
		template: &SmolStr,
		metadata: &Map,
		parts: &RequestParts,
		params: ThreadEntryParams,
	) -> Result<ThreadEntry> {
		let mut stamp = Map::default();
		for (key, value) in metadata.iter() {
			let value = match value {
				Value::Str(text) => {
					Value::new(crate::table::request_post::fill(text, parts)?)
				}
				other => other.clone(),
			};
			stamp.insert(key.clone(), value);
		}
		stamp.insert("template", template.as_str());
		ThreadEntry {
			continues: !metadata.is_empty(),
			metadata: stamp,
			params,
		}
		.xok()
	}

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
	/// close and flush every thread and answer.
	async fn run(
		root: &AsyncEntity,
		template: &str,
		entry: ThreadEntry,
	) -> Result<Response> {
		let threads = RunThread::enter_all(root, &entry).await?;
		let behavior = root
			.with_state::<Query<&Children>, _>(|root, children| {
				children.get(root).ok().and_then(|children| {
					(children.len() == 1).then(|| children[0])
				})
			})
			.await?
			.ok_or_else(|| {
				bevyhow!(
					"the template `{template}` must build exactly one root"
				)
			})?;
		let thread = *threads.first().ok_or_else(|| {
			bevyhow!("the template `{template}` builds no `<Thread>`")
		})?;
		let outcome =
			root.world().entity(behavior).call::<(), Outcome>(()).await;
		RunThread::close_all(&root.world(), &threads, &outcome).await?;
		let outcome = outcome?;
		let last_text = root
			.world()
			.with(move |world: &mut World| Self::last_text(world, thread))
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

	/// The last completed text an acting actor of `thread` posted: an actor
	/// with behavior beneath it ([`ActorRef`]) other than a person, so an
	/// agent's reply or a loop's decision answers, never the owner's turn.
	fn last_text(world: &mut World, thread: Entity) -> Option<String> {
		let acting = world
			.with_state::<(Query<&Children>, Query<&ActorRef>), _>(
				|(children, actor_refs)| {
					children
						.iter_descendants(thread)
						.filter_map(|entity| actor_refs.get(entity).ok())
						.map(|actor_ref| **actor_ref)
						.collect::<Vec<_>>()
				},
			);
		let window = world.get::<ThreadWindow>(thread)?;
		window
			.authored_posts()
			.filter(|authored| {
				acting.contains(&authored.actor.id())
					&& authored.actor.kind() != ActorKind::User
					&& !authored.post.in_progress()
			})
			.filter_map(|authored| match authored.post.as_agent_post() {
				AgentPost::Text(text) => Some(text.to_string()),
				_ => None,
			})
			.last()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_router::prelude::*;

	/// The thread a review builds per request: pinned actors, the reviewed
	/// document in its seed, a reading agent with tools of its own that reads once then
	/// answers, its records kept under `threads`.
	const REVIEW_THREAD: &str = r#"
<RepeatWhileFunctionCallOutput>
	<Thread {(Sequence, MountThreadStore{path:"threads"})}>
		<CreateActor name="User" kind="User" id=1><RequestPost text="review {document}"/></CreateActor>
		<CreateActor name="Agent" kind="Agent" id=2 {MockPostStreamer{script:[Call{tool:"read-blob", arguments:{path:"notes.md"}}]}}>
			<Router><StoreToolset read_only=true/></Router>
		</CreateActor>
	</Thread>
</RepeatWhileFunctionCallOutput>
"#;

	/// A thread delegating a review: its agent requests the host's review
	/// route as a tool, naming the post that asked for it as a body field.
	const DELEGATE_THREAD: &str = r#"
<RepeatWhileFunctionCallOutput>
	<Thread {(Sequence, MountThreadStore{path:"delegates"})}>
		<CreateActor name="Agent" kind="Agent" id=3 {MockPostStreamer{script:[Call{tool:"review", arguments:{
			document:"product",
			spawned_by:"00000000-0000-0000-0000-000000000001/00000000-0000-0000-0000-000000000002",
		}}]}}>
			<Router><HostTool path="review"/></Router>
		</CreateActor>
	</Thread>
</RepeatWhileFunctionCallOutput>
"#;

	/// The post [`DELEGATE_THREAD`]'s call names.
	fn delegating_post() -> PostRef {
		PostRef::new(ThreadId::from_u128(1), PostId::from_u128(2))
	}

	/// A thread whose behavior errors once its session opens.
	const FAILING_THREAD: &str = r#"
<Thread {(Sequence, MountThreadStore{path:"threads"})}>
	<CreateActor name="Agent" kind="Agent" id=2 {Bail}/>
</Thread>
"#;

	/// A behavior that errors.
	async fn bail(_cx: ActionContext) -> Result<Outcome> { bevybail!("boom") }
	#[derive(Component, Reflect, Default)]
	#[reflect(Component, Default)]
	#[require(Action<(), Outcome> = Action::new_async(bail))]
	struct Bail;

	/// A host url space holding the review route, spread with `review`, over
	/// an in-memory store.
	fn host(review: &str) -> (App, Entity, BlobStore) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_type::<ThreadPlugin>()
			.register_type::<Bail>();
		let mut registry = app
			.world_mut()
			.get_resource_or_init::<BsxTemplateRegistry>();
		for (name, source) in [
			("ReviewThread", REVIEW_THREAD),
			("DelegateThread", DELEGATE_THREAD),
			("FailingThread", FAILING_THREAD),
		] {
			registry.insert_source(name, source).unwrap();
		}
		let store = BlobStore::temp();
		let router = BsxTemplate::parse_entry(
			app.world(),
			&format!(
				r#"<Router>
	<Route path="review/:document" {{{review}}}/>
	<Route path="delegate" {{ThreadRoute{{template:"DelegateThread"}}}}/>
	<Route path="fail" {{ThreadRoute{{template:"FailingThread"}}}}/>
</Router>"#
			),
		)
		.unwrap()
		.spawn(app.world_mut())
		.unwrap();
		app.world_mut().entity_mut(router).insert(store.clone());
		(app, router, store)
	}

	/// The review route continuing by its document.
	const BY_DOCUMENT: &str = r#"ThreadRoute{template:"ReviewThread", metadata:{document:"{document}"}}"#;

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

	/// The records under `threads` in `store`.
	fn records(store: &BlobStore) -> BlobThreadStore {
		BlobThreadStore::new(store.with_subdir(RelPath::new("threads")))
	}

	/// Every stored thread's text posts, the tool calls and sessions aside,
	/// sorted.
	async fn stored(store: &BlobStore) -> Vec<Vec<String>> {
		let records = records(store);
		let mut threads = Vec::new();
		for thread in records.threads().await.unwrap() {
			threads.push(texts(
				&records.thread_posts(thread.id(), None).await.unwrap(),
			));
		}
		threads.sort();
		threads
	}

	/// The text posts of `posts`, the tool calls and sessions aside.
	fn texts(posts: &[Post]) -> Vec<String> {
		posts
			.iter()
			.filter(|post| matches!(post.as_agent_post(), AgentPost::Text(_)))
			.map(|post| post.to_string())
			.collect()
	}

	/// The one stored thread.
	async fn only_thread(store: &BlobStore) -> (Thread, Vec<Post>) {
		let records = records(store);
		let threads = records.threads().await.unwrap();
		threads.len().xpect_eq(1);
		let posts = records.thread_posts(threads[0].id(), None).await.unwrap();
		(threads[0].clone(), posts)
	}

	/// The thread answers with its agent's last text, and is gone once it has,
	/// its posts in the store and the host's route tree untouched.
	#[beet_core::test]
	async fn answers_then_despawns_its_thread() {
		let (mut app, router, store) = host(BY_DOCUMENT);
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

	/// Two requests with the same filled metadata continue one thread, an
	/// unchanged seed appending nothing, and another value starts its own,
	/// stamped with its entries and the template.
	#[beet_core::test]
	async fn continues_by_its_metadata() {
		let (mut app, router, store) = host(BY_DOCUMENT);
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
		records(&store)
			.threads()
			.await
			.unwrap()
			.iter()
			.any(|thread| {
				thread.metadata().get("document").ok()
					== Some(&Value::new("market"))
					&& thread.metadata().get("template").ok()
						== Some(&Value::new("ReviewThread"))
			})
			.xpect_true();
	}

	/// A route declaring no metadata starts a thread every request.
	#[beet_core::test]
	async fn no_metadata_is_fresh_every_time() {
		let (mut app, router, store) =
			host(r#"ThreadRoute{template:"ReviewThread"}"#);
		for _ in 0..2 {
			review(&mut app, router, Request::get("review/product")).await;
		}
		stored(&store).await.len().xpect_eq(2);
	}

	/// `--new` starts a thread stamped the same, which the next request
	/// continues, and `--thread` continues a stored thread by id.
	#[beet_core::test]
	async fn new_and_thread_pick_the_thread() {
		let (mut app, router, store) = host(BY_DOCUMENT);
		review(&mut app, router, Request::get("review/product")).await;
		let (first, _) = only_thread(&store).await;
		for request in [
			Request::get("review/product").with_param("new", "true"),
			Request::get("review/product"),
			Request::get("review/product")
				.with_param("thread", &first.id().to_string()),
		] {
			review(&mut app, router, request)
				.await
				.0
				.xpect_eq(StatusCode::OK);
		}
		let reply = "you said: review product".to_string();
		let seed = "review product".to_string();
		stored(&store).await.xpect_eq(vec![
			vec![seed.clone(), reply.clone(), reply.clone()],
			vec![seed, reply.clone(), reply],
		]);
		review(
			&mut app,
			router,
			Request::get("review/product").with_param("thread", "nope"),
		)
		.await
		.0
		.xpect_eq(StatusCode::BAD_REQUEST);
	}

	/// Each request is a session bracketed by its posts, the second naming
	/// its own cause while the thread keeps its creator, and model input
	/// carries neither.
	#[beet_core::test]
	async fn sessions_are_bracketed_by_posts() {
		let (mut app, router, store) = host(BY_DOCUMENT);
		let creator = delegating_post();
		let second = PostRef::new(ThreadId::from_u128(1), PostId::from_u128(3));
		for spawned_by in [&creator, &second] {
			review(
				&mut app,
				router,
				Request::get("review/product")
					.with_param("spawned-by", &spawned_by.to_string()),
			)
			.await
			.0
			.xpect_eq(StatusCode::OK);
		}
		let (thread, posts) = only_thread(&store).await;
		thread.spawned_by().xpect_eq(Some(&creator));
		posts
			.iter()
			.filter_map(SessionOpened::of)
			.map(|opened| opened.spawned_by)
			.collect::<Vec<_>>()
			.xpect_eq(vec![Some(creator), Some(second)]);
		let closed = posts
			.iter()
			.filter_map(SessionClosed::of)
			.collect::<Vec<_>>();
		closed
			.iter()
			.map(|closed| closed.outcome)
			.collect::<Vec<_>>()
			.xpect_eq(vec![Outcome::PASS, Outcome::PASS]);
		// each totals the usage of its own session's two mock responses
		for closed in &closed {
			closed.usage.input_tokens.xpect_eq(20);
		}
		// the scripted agent answered the review both times, never a session
		let mut window = ThreadWindow::new();
		for actor in records(&store).actors().await.unwrap() {
			window.insert_actor(actor);
		}
		for post in posts {
			window.upsert_post(post);
		}
		window
			.model_posts(None)
			.iter()
			.any(|view| view.post.intent().is_session())
			.xpect_false();
	}

	/// A request without `spawned_by`, ie a unit run by hand, records no
	/// spawn, and one forwarded by an agent's tool call carries it as a body
	/// field of the call.
	#[beet_core::test]
	async fn spawned_by_travels_as_a_body_field() {
		let (mut app, router, store) = host(BY_DOCUMENT);
		review(&mut app, router, Request::get("review/product")).await;
		only_thread(&store).await.0.spawned_by().xpect_none();

		let (mut app, router, store) = host(BY_DOCUMENT);
		review(&mut app, router, Request::get("delegate"))
			.await
			.0
			.xpect_eq(StatusCode::OK);
		only_thread(&store)
			.await
			.0
			.spawned_by()
			.xpect_eq(Some(&delegating_post()));
	}

	/// A session whose behavior errors still closes, as a fail, and its
	/// records reach the store.
	#[beet_core::test]
	async fn an_erroring_session_still_closes() {
		let (mut app, router, store) = host(BY_DOCUMENT);
		review(&mut app, router, Request::get("fail"))
			.await
			.1
			.xpect_contains("boom");
		only_thread(&store)
			.await
			.1
			.iter()
			.filter_map(SessionClosed::of)
			.map(|closed| closed.outcome)
			.collect::<Vec<_>>()
			.xpect_eq(vec![Outcome::FAIL]);
	}
}
