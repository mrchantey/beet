//! [`ThreadRoute`], a route answering each request with a thread built for it.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// A thread run as a function: built for one request from the document or
/// template it names, run to its outcome, answered and despawned: a request in, an
/// answer out, nobody typing. It is the narrow case of a thread; one a person
/// sits in is held by a [`RunThread`].
///
/// ```bsx
/// <Route path="review/:document" {ThreadRoute{source:Document("templates/ReviewThread.bsx"), message:"review {document}"}}/>
/// <!-- templates/ReviewThread.bsx -->
/// <Thread {(Sequence, MountThreadStore{path:"threads/review/{document}"})}> ..actors.. </Thread>
/// ```
///
/// The words are the model's, defined once in the [`table`](crate::table)
/// module docs. On every request it:
///
/// 1. builds a fresh thread from what it names ([`TemplateSource`], as a
///    [`Layout`] does), with the request on the [`RequestContextStack`] for
///    the build, then attaches it beneath the route, where it resolves the ancestor stores and the surrounding url
///    space as if authored there. It is built detached first, so the
///    surrounding [`RouteTree`] never rebuilds for it;
/// 2. enters it ([`Thread::enter`]), the template's [`MountThreadStore`]
///    path, filled from the request, naming the thread it continues, and
///    the request's [`ThreadEntryParams`] overriding it; then posts
///    `message`, each `{param}` filled from the request, by `actor`, else
///    the first User actor the scene authors;
/// 3. runs the template's behavior to its [`Outcome`], writing nothing for
///    its end;
/// 4. waits for every completed post to reach its [`ThreadStore`]
///    ([`ThreadStore::flush`]), however the behavior ended, then despawns
///    the thread, as it does when the request is dropped;
/// 5. answers the last completed text of the thread's acting actors as the
///    body on a pass, and a `500` naming the failure on a fail.
///
/// A failed build, a refused entry or a behavior that errors fail the
/// request the same way, the thread despawned all the same. A template
/// holding a [`UserInput`] is refused, naming its actor, and a request for a
/// thread whose canonical window is live answers `409`.
#[action(no_default)]
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn ThreadRoute(
	/// The document or rust template building the thread.
	#[field]
	source: TemplateSource,
	/// The message each request posts once the thread is entered, each
	/// `{param}` filled from the request. None posts nothing.
	#[field]
	#[reflect(default)]
	message: Option<SmolStr>,
	/// The message's author, by id, ie `actor:"ann"`, else the first User
	/// actor the scene authors.
	#[field]
	#[reflect(default)]
	actor: Option<ActorId>,
	entry: Params<ThreadEntryParams>,
	cx: ActionContext<Request>,
) -> Result<Response> {
	let route = cx.caller.clone();
	let world = route.world().clone();
	let mut entry = ThreadEntry::new(
		cx.input.parts().clone(),
		ThreadEntryParams::clone(&entry),
	);
	if let Some(message) = &message {
		entry = entry.with_message(message, actor)?;
	}
	let source = source.load(&route).await?;
	let function = ThreadFunction::build(
		&world,
		route.id(),
		Some(route.id()),
		source,
		cx.input.parts().clone(),
	)
	.await?;
	let answer = ThreadRoute::answer(&function, entry).await;
	// torn down however the run ended, the response answered after
	function.despawn().await;
	answer
}

impl ThreadRoute {
	/// Run `function` with `entry` and answer: the last text of the
	/// thread's acting actors on a pass, a `500` naming the failure on a
	/// fail.
	async fn answer(
		function: &ThreadFunction,
		entry: ThreadEntry,
	) -> Result<Response> {
		let template = function.name().to_string();
		let ThreadOutcome { outcome, thread } = function.run(entry).await?;
		let last_text = function
			.world()
			.with(move |world: &mut World| {
				ThreadFunction::last_text(world, thread)
			})
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
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_router::prelude::*;
	use beet_scene::prelude::*;

	/// The thread a review builds per request, one per document: two people
	/// and a reading agent with tools of its own that reads
	/// once then answers.
	const REVIEW_THREAD: &str = r#"
<RepeatWhileFunctionCallOutput>
	<Thread {(Sequence, MountThreadStore{path:"threads/{document}"})}>
		<CreateActor name="Author" kind="User"/>
		<CreateActor name="Editor" kind="User"/>
		<CreateActor name="Agent" kind="Agent" {MockPostStreamer{script:[Call{tool:"read-blob", arguments:{path:"notes.md"}}]}}>
			<Router><StoreToolset read_only=true/></Router>
		</CreateActor>
	</Thread>
</RepeatWhileFunctionCallOutput>
"#;

	/// A thread delegating a review: its agent requests the surrounding review
	/// route as a tool, naming the post that asked for it as a body field.
	const DELEGATE_THREAD: &str = r#"
<RepeatWhileFunctionCallOutput>
	<Thread {(Sequence, MountThreadStore{path:"delegates"})}>
		<CreateActor name="Agent" kind="Agent" {MockPostStreamer{script:[Call{tool:"review", arguments:{
			document:"product",
			spawned_by:"2222222222222/2222222222223",
		}}]}}>
			<Router><RouteTool path="review"/></Router>
		</CreateActor>
	</Thread>
</RepeatWhileFunctionCallOutput>
"#;

	/// The post [`DELEGATE_THREAD`]'s call names.
	fn delegating_post() -> PostRef {
		PostRef::parse("2222222222222/2222222222223").unwrap()
	}

	/// A thread with no person in it, whose behavior errors once entered.
	const FAILING_THREAD: &str = r#"
<Thread {(Sequence, MountThreadStore{path:"failures"})}>
	<CreateActor name="Agent" kind="Agent" {Bail}/>
</Thread>
"#;

	/// A thread whose agent holds its turn until the test releases it.
	const HOLDING_THREAD: &str = r#"
<Thread {(Sequence, MountThreadStore{path:"held"})}>
	<CreateActor name="Agent" kind="Agent" {Hold}/>
</Thread>
"#;

	/// A thread a person types into.
	const SITTING_THREAD: &str = r#"
<Thread {Sequence}>
	<CreateActor name="Owner" kind="User" {UserInput}/>
	<CreateActor name="Agent" kind="Agent" {MockPostStreamer}/>
</Thread>
"#;

	/// A behavior that errors.
	async fn bail(_cx: ActionContext) -> Result<Outcome> { bevybail!("boom") }
	#[derive(Component, Reflect, Default)]
	#[reflect(Component, Default)]
	#[require(Action<(), Outcome> = Action::new_async(bail))]
	struct Bail;

	/// Lets every [`Hold`] turn pass.
	#[derive(Resource)]
	struct Release;

	/// A turn waiting for [`Release`].
	async fn hold(cx: ActionContext) -> Result<Outcome> {
		while !cx
			.world()
			.with(|world: &mut World| world.contains_resource::<Release>())
			.await
		{
			async_ext::yield_now().await;
		}
		Outcome::PASS.xok()
	}
	#[derive(Component, Reflect, Default)]
	#[reflect(Component, Default)]
	#[require(Action<(), Outcome> = Action::new_async(hold))]
	struct Hold;

	/// A url space over an in-memory store holding each thread document, its
	/// routes running each.
	async fn url_space() -> (App, Entity, BlobStore) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_types::<ThreadPlugin>()
			.register_type::<Bail>()
			.register_type::<Hold>();
		let store = BlobStore::temp();
		for (name, source) in [
			("ReviewThread", REVIEW_THREAD),
			("DelegateThread", DELEGATE_THREAD),
			("FailingThread", FAILING_THREAD),
			("HoldingThread", HOLDING_THREAD),
			("SittingThread", SITTING_THREAD),
		] {
			store
				.insert(&FwdPath::new(format!("{name}.bsx")).unwrap(), source)
				.await
				.unwrap();
		}
		let body = BsxParser::body(
			app.world(),
			r#"<Router>
	<Route path="review/:document" {ThreadRoute{source:Document("ReviewThread.bsx"), message:"review {document}"}}/>
	<Route path="edit/:document" {ThreadRoute{source:Document("ReviewThread.bsx"), message:"edit {document}", actor:"editor"}}/>
	<Route path="delegate" {ThreadRoute{source:Document("DelegateThread.bsx")}}/>
	<Route path="fail" {ThreadRoute{source:Document("FailingThread.bsx")}}/>
	<Route path="hold" {ThreadRoute{source:Document("HoldingThread.bsx")}}/>
	<Route path="lonely" {ThreadRoute{source:Document("FailingThread.bsx"), message:"anyone?"}}/>
	<Route path="sit" {ThreadRoute{source:Document("SittingThread.bsx")}}/>
</Router>"#,
		)
		.unwrap();
		let router = app.world_mut().spawn_template(body).unwrap().id();
		app.world_mut().entity_mut(router).insert((
			store.clone(),
			JsonTableStore::over(store.clone()).unwrap(),
		));
		(app, router, store)
	}

	/// Request `request` of `router`, answering its status and body.
	async fn review(
		app: &mut App,
		router: Entity,
		request: Request,
	) -> (StatusCode, String) {
		let response =
			app.world_mut().entity_mut(router).exchange(request).await;
		(response.status(), response.text().await.unwrap())
	}

	/// The records under `path` in `store`.
	fn thread_store(store: &BlobStore, path: &str) -> ThreadStore {
		ThreadStore::new(
			JsonTableStore::over(store.clone()).unwrap(),
			FwdPath::new(path).unwrap(),
		)
		.unwrap()
	}

	/// The text posts of `posts`, the tool calls and joins aside.
	fn texts(posts: &[Post]) -> Vec<String> {
		posts
			.iter()
			.filter(|post| {
				matches!(post.as_agent_post(), Some(AgentPost::Text(_)))
			})
			.map(|post| post.to_string())
			.collect()
	}

	/// The one thread stored under `path`.
	async fn only_thread(store: &BlobStore, path: &str) -> (Thread, Vec<Post>) {
		let records = thread_store(store, path);
		let threads = records.threads().await.unwrap();
		threads.len().xpect_eq(1);
		let posts = records.posts(threads[0].id()).await.unwrap();
		(threads[0].clone(), posts)
	}

	/// The answer is what is shown: an acting agent's text it deleted is
	/// never the answer, its text before that is.
	#[beet_core::test]
	async fn the_answer_leaves_out_what_was_deleted() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_types::<ThreadPlugin>();
		let thread = app
			.world_mut()
			.spawn((Thread::default(), children![
				Actor::user(),
				(Actor::agent(), MockPostStreamer::default()),
			]))
			.id();
		app.update();
		let id = app.world().get::<Thread>(thread).unwrap().id();
		let said = |text: &str| {
			AgentPost::new_text(ActorId::AGENT, id, text, PostStatus::Completed)
		};
		let secret = said("DELETED-SECRET");
		for post in [said("the answer"), secret.clone()] {
			app.world_mut()
				.get_mut::<ThreadRecords>(thread)
				.unwrap()
				.upsert_post(post);
		}
		app.world_mut()
			.run_async_local_then(async move |world: AsyncWorld| {
				DeletePost::new(ActorId::AGENT, secret.id())
					.apply(&world.entity(thread))
					.await
			})
			.await
			.unwrap();
		ThreadFunction::last_text(app.world_mut(), thread)
			.xpect_eq(Some("the answer".to_string()));
	}

	/// The thread answers with its agent's last text, and is gone once it has,
	/// its posts in the store and the surrounding route tree untouched.
	#[beet_core::test]
	async fn answers_then_despawns_its_thread() {
		let (mut app, router, store) = url_space().await;
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
		texts(&only_thread(&store, "threads/product").await.1).xpect_eq(vec![
			"review product".to_string(),
			"you said: review product".to_string(),
		]);
		app.world()
			.entity(router)
			.get_ref::<RouteTree>()
			.unwrap()
			.last_changed()
			.xpect_eq(tree);
	}

	/// Two requests naming the same document continue one thread, each
	/// posting its own message, and another document keeps its own.
	#[beet_core::test]
	async fn continues_by_its_path() {
		let (mut app, router, store) = url_space().await;
		for path in ["review/product", "review/product", "review/market"] {
			review(&mut app, router, Request::get(path))
				.await
				.0
				.xpect_eq(StatusCode::OK);
		}
		texts(&only_thread(&store, "threads/product").await.1).xpect_eq(
			["review product", "you said: review product"]
				.repeat(2)
				.into_iter()
				.map(str::to_string)
				.collect::<Vec<_>>(),
		);
		texts(&only_thread(&store, "threads/market").await.1).xpect_eq(vec![
			"review market".to_string(),
			"you said: review market".to_string(),
		]);
	}

	/// A request's message lands once per request, after its entry's joins,
	/// by the named actor, else the first User actor the scene authors.
	#[beet_core::test]
	async fn posts_the_message_by_its_actor() {
		let (mut app, router, store) = url_space().await;
		for path in ["review/product", "edit/product"] {
			review(&mut app, router, Request::get(path))
				.await
				.0
				.xpect_eq(StatusCode::OK);
		}
		let (_, posts) = only_thread(&store, "threads/product").await;
		let messages = posts
			.iter()
			.filter(|post| {
				post.to_string() == "review product"
					|| post.to_string() == "edit product"
			})
			.map(|post| (post.to_string(), post.author().to_string()))
			.collect::<Vec<_>>();
		messages.xpect_eq(vec![
			("review product".to_string(), "author".to_string()),
			("edit product".to_string(), "editor".to_string()),
		]);
		// each message directly follows its own entry's joins
		for (index, post) in posts.iter().enumerate() {
			if SessionOpened::opens(post)
				&& !SessionOpened::opens(&posts[index + 1])
			{
				matches!(
					posts[index + 1].to_string().as_str(),
					"review product" | "edit product"
				)
				.xpect_true();
			}
		}
	}

	/// A message with no actor named fails the request when the thread holds
	/// no User actor.
	#[beet_core::test]
	async fn a_message_needs_an_author() {
		let (mut app, router, _) = url_space().await;
		let (status, text) =
			review(&mut app, router, Request::get("lonely")).await;
		status.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
		text.xpect_contains("holds no User actor");
	}

	/// The request's params pick the thread: `--new` creates one and
	/// `--thread` continues a stored one by id, a malformed id refused.
	#[beet_core::test]
	async fn its_params_pick_the_thread() {
		let (mut app, router, store) = url_space().await;
		review(&mut app, router, Request::get("review/product")).await;
		let (first, _) = only_thread(&store, "threads/product").await;
		for request in [
			Request::get("review/product").with_param("new", "true"),
			Request::get("review/product")
				.with_param("thread", &first.id().to_string()),
		] {
			review(&mut app, router, request)
				.await
				.0
				.xpect_eq(StatusCode::OK);
		}
		let records = thread_store(&store, "threads/product");
		records.threads().await.unwrap().len().xpect_eq(2);
		texts(&records.posts(first.id()).await.unwrap())
			.len()
			.xpect_eq(4);
		review(
			&mut app,
			router,
			Request::get("review/product").with_param("thread", "nope"),
		)
		.await
		.0
		.xpect_eq(StatusCode::BAD_REQUEST);
	}

	/// Each request joins every actor of its scene and nothing marks its
	/// end, the second request's joins naming its own cause while the thread
	/// keeps its creator, and usage stays on each response's metadata.
	#[beet_core::test]
	async fn each_request_joins_its_actors() {
		let (mut app, router, store) = url_space().await;
		let creator = delegating_post();
		let second = PostRef::new(ThreadId::now(), PostId::now());
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
		let (thread, posts) = only_thread(&store, "threads/product").await;
		thread.spawned_by().xpect_eq(Some(&creator));
		// each entry joins every actor of the scene, each by itself
		posts
			.iter()
			.filter_map(|post| {
				post.log::<SessionOpened>()
					.map(|opened| (post.author().clone(), opened.spawned_by))
			})
			.collect::<Vec<_>>()
			.xpect_eq(
				[&creator, &second]
					.into_iter()
					.flat_map(|cause| {
						["author", "editor", "agent"].map(|actor| {
							(ActorId::from_static(actor), Some(cause.clone()))
						})
					})
					.collect::<Vec<_>>(),
			);
		// a join is the only log post: no request wrote one for its end
		posts
			.iter()
			.filter(|post| post.is_log())
			.all(SessionOpened::opens)
			.xpect_true();
		// two mock responses a request, each keeping its own usage
		let records = thread_store(&store, "threads/product")
			.open(&thread, Reach::Whole)
			.await
			.unwrap();
		records
			.metas()
			.filter(|meta| {
				meta.usage.is_some_and(|usage| usage.input_tokens > 0)
			})
			.count()
			.xpect_eq(4);
		// the model reads each request's joins as one note
		records
			.model_posts(&records.shown(..), &ActorId::AGENT, None)
			.iter()
			.filter(|item| matches!(item, ModelPost::Note(_)))
			.count()
			.xpect_eq(2);
	}

	/// A request without `spawned_by`, ie a unit run by hand, records no
	/// spawn, and one forwarded by an agent's tool call carries it as a body
	/// field of the call.
	#[beet_core::test]
	async fn spawned_by_travels_as_a_body_field() {
		let (mut app, router, store) = url_space().await;
		review(&mut app, router, Request::get("review/product")).await;
		only_thread(&store, "threads/product")
			.await
			.0
			.spawned_by()
			.xpect_none();

		let (mut app, router, store) = url_space().await;
		review(&mut app, router, Request::get("delegate"))
			.await
			.0
			.xpect_eq(StatusCode::OK);
		only_thread(&store, "threads/product")
			.await
			.0
			.spawned_by()
			.xpect_eq(Some(&delegating_post()));
	}

	/// A thread whose behavior errors still reaches its store, with no post
	/// for its end.
	#[beet_core::test]
	async fn an_erroring_thread_still_flushes() {
		let (mut app, router, store) = url_space().await;
		review(&mut app, router, Request::get("fail"))
			.await
			.1
			.xpect_contains("boom");
		only_thread(&store, "failures")
			.await
			.1
			.iter()
			.map(|post| (post.author().clone(), SessionOpened::opens(post)))
			.collect::<Vec<_>>()
			.xpect_eq(vec![(ActorId::AGENT, true)]);
	}

	/// A request for a thread whose canonical window is live, a request
	/// before it still running, answers `409`, and the first answers once its
	/// turn passes.
	#[beet_core::test]
	async fn a_live_thread_answers_conflict() {
		let (mut app, router, _) = url_space().await;
		let (first, second) = app
			.world_mut()
			.entity_mut(router)
			.run_async_then(async move |router: AsyncEntity| {
				let world = router.world().clone();
				let second = async {
					// the first request's window is live once entered
					while !world
						.with(|world: &mut World| {
							world
								.query_filtered::<(), With<ThreadStore>>()
								.iter(world)
								.next()
								.is_some()
						})
						.await
					{
						async_ext::yield_now().await;
					}
					let response = router.exchange(Request::get("hold")).await;
					world
						.with(|world: &mut World| {
							world.insert_resource(Release)
						})
						.await;
					response
				};
				futures::join!(router.exchange(Request::get("hold")), second)
			})
			.await;
		first.status().xpect_eq(StatusCode::OK);
		second.status().xpect_eq(StatusCode::CONFLICT);
		second
			.text()
			.await
			.unwrap()
			.xpect_contains("already runs its behavior in the window");
	}

	/// A request dropped while its thread runs, ie its client gone,
	/// despawns the thread all the same.
	#[beet_core::test]
	async fn a_dropped_request_despawns_its_thread() {
		let (mut app, router, _) = url_space().await;
		app.world_mut()
			.entity_mut(router)
			.run_async_then(async move |router: AsyncEntity| {
				let world = router.world().clone();
				let live = async {
					while !world
						.with(|world: &mut World| {
							world
								.query_filtered::<(), With<ThreadStore>>()
								.iter(world)
								.next()
								.is_some()
						})
						.await
					{
						async_ext::yield_now().await;
					}
				};
				let request = router.exchange(Request::get("hold"));
				futures::future::select(
					core::pin::pin!(request),
					core::pin::pin!(live),
				)
				.await;
			})
			.await;
		let threads = |app: &mut App| {
			app.world_mut()
				.query_filtered::<(), With<Thread>>()
				.iter(app.world())
				.count()
		};
		for _ in 0..50 {
			if threads(&mut app) == 0 {
				break;
			}
			app.update();
		}
		threads(&mut app).xpect_eq(0);
	}

	/// A template a person types into is refused, naming the actor.
	#[beet_core::test]
	async fn refuses_a_person_typing() {
		let (mut app, router, _) = url_space().await;
		let (status, text) =
			review(&mut app, router, Request::get("sit")).await;
		status.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
		text.xpect_contains("holds a `UserInput` on the actor `Owner`");
	}
}
