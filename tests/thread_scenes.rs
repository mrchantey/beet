//! Each thread example's `.bsx` author scene parses + reduces into the expected
//! `ThreadRecords` + behavior, guarding the shipped scenes against drift. The
//! agent scenes use OpenAi, whose streamer resolves an auth env at build, so a
//! dummy key is set; no network is touched (reduction only).
beet::test_main!();

use beet::prelude::*;
use std::sync::Once;

/// Set a dummy auth env once so `{ModelStreamer{provider:OpenAi}}` builds during
/// reduction without a real key (no request is ever made). Nothing here boots, so
/// the scenes' servers stay inert: a `TuiServer` only takes the terminal on its
/// start walk, which reduction never fires.
fn ensure_auth_env() {
	static INIT: Once = Once::new();
	INIT.call_once(|| unsafe {
		env_ext::set_var("OPENAI_API_KEY", "test-dummy-key").unwrap();
	});
}

/// A stand-in for `tool_call`'s inline tool, so `<AgentChoiceAction/>` resolves
/// when reducing its scene here (the example defines an identically-shaped one).
#[action(pure, route = "make-choice")]
#[derive(Component, Reflect)]
#[reflect(Component)]
fn AgentChoiceAction(cx: ActionContext<ChoiceInput>) -> String {
	cx.catchphrase.clone()
}
#[derive(Reflect, serde::Serialize, serde::Deserialize)]
struct ChoiceInput {
	/// a line of dialog
	catchphrase: String,
}

/// Parse + spawn + reduce a scene, returning the reduced app.
///
async fn reduce(source: &str) -> App {
	ensure_auth_env();
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.install_types::<(
			ThreadPlugin,
			// registers `{UserInput}` and the `CreatePostForm` template so the
			// interactive chat scenes resolve
			ThreadUiPlugin,
			// resolves each scene's `<RequireCfg cfg=".."/>` tag
			CrateCheckPlugin,
		)>()
		.register_type::<AgentChoiceAction>();
	app.world_mut().spawn(cli_registration());
	AsyncRunner::settle_async_tasks(app.world_mut()).await;
	BsxParser::spawn(app.world_mut(), source).unwrap();
	ThreadRecords::reduce_now(app.world_mut());
	app.world_mut().flush();
	app
}

/// A stand-in for the `beet` binary the scenes check against, declaring the cli
/// feature names they name.
///
/// Hand-built rather than `crate_registration!`, whose `cfg!` reads the *calling*
/// crate: `tui`/`ssh` are beet-cli features forwarding to this crate's
/// `tui_server`/`ssh_tui`, which this test is gated on, so the declaration holds.
/// `extra` supplies `tool_call`'s inline tool, stubbed here.
fn cli_registration() -> CrateRegistration {
	["thread", "tui", "ssh", "extra"]
		.into_iter()
		.fold(
			CrateRegistration::new("beet", env!("CARGO_PKG_VERSION")),
			CrateRegistration::with_feature,
		)
		.with_skip_prefix()
}

/// No `<RequireCfg>` failed: its observer reports a missing feature by writing
/// `AppExit::error()`, which nothing else here reads, so an undeclared
/// requirement would otherwise pass silently.
fn assert_crate_check_passed(app: &mut App) {
	app.world_mut()
		.run_system_once(|mut exits: MessageReader<AppExit>| {
			exits.read().count()
		})
		.unwrap()
		.xpect_eq(0);
}

/// Every scene declares the same served shape: a root carrying the local TUI
/// server (default-booting) plus the two opt-in ones, an `AppShell`-wrapped
/// router, one `FixedPage` route, and a `ThreadView` whose `$thread` reference
/// resolved to the scene's own thread. Asserted per scene, since an unresolved
/// tag would otherwise reduce to a thread that renders nowhere.
fn assert_served_shape(app: &mut App) {
	let world = app.world_mut();
	// the local terminal boots by default, http and ssh only when named
	world
		.query_once::<&TuiServer>()
		.into_iter()
		.map(|server| server.default_boot)
		.collect::<Vec<_>>()
		.xpect_eq(vec![true]);
	world
		.query_once::<&HttpServer>()
		.into_iter()
		.map(|server| server.default_boot)
		.collect::<Vec<_>>()
		.xpect_eq(vec![false]);
	world
		.query_once::<&SshTuiServer>()
		.into_iter()
		.map(|server| server.default_boot)
		.collect::<Vec<_>>()
		.xpect_eq(vec![false]);
	// one layout-wrapped router serving one persistent page
	world
		.query_filtered_once::<Entity, (With<Router>, With<Layout>)>()
		.len()
		.xpect_eq(1);
	world
		.query_filtered_once::<Entity, With<FixedPage>>()
		.len()
		.xpect_eq(1);
	// the view's `$thread` reference resolved to the scene's thread
	let thread = world
		.query_filtered_once::<Entity, With<Thread>>()
		.into_iter()
		.next()
		.unwrap();
	let views = world
		.query_filtered_once::<&OfThread, With<ThreadView>>()
		.into_iter()
		.map(|of_thread| of_thread.thread())
		.collect::<Vec<_>>();
	views.xpect_eq(vec![thread]);
}

/// The reduced thread's `(actor count, seed-post count)`, its joins aside.
fn record_counts(app: &mut App) -> (usize, usize) {
	let mut threads = app.world_mut().query_filtered::<Entity, With<Thread>>();
	let thread = threads.iter(app.world()).next().unwrap();
	let records = app.world().get::<ThreadRecords>(thread).unwrap();
	(records.actors().len(), records.content_posts().count())
}

/// Declare one temp table store on every thread mounting one, nearer than the
/// one its example declares, which is on disk, answering those threads.
fn declare_temp_tables(app: &mut App) -> Vec<Entity> {
	let tables = TableStore::new(InMemoryTableStore::new());
	let threads = app
		.world_mut()
		.query_filtered_once::<Entity, With<MountThreadStore>>()
		.into_iter()
		.collect::<Vec<_>>();
	for thread in &threads {
		app.world_mut().entity_mut(*thread).insert(tables.clone());
	}
	threads
}

/// Enter every thread mounting a store, over a temp store, as its start
/// would: a mounted thread reduces once its entry knows whether it creates
/// its thread.
async fn enter_mounted(app: &mut App) {
	let threads = declare_temp_tables(app);
	app.world_mut()
		.run_async_local_then(async move |world: AsyncWorld| -> Result {
			for thread in threads {
				Thread::enter(world.clone(), thread, default()).await?;
			}
			Ok(())
		})
		.await
		.unwrap();
}

/// The kinds of every actor in the reduced records, so a scene's `kind="..."`
/// attributes are verified to resolve (not silently default to `Agent`).
fn actor_kinds(app: &mut App) -> Vec<ActorKind> {
	let mut threads = app.world_mut().query_filtered::<Entity, With<Thread>>();
	let thread = threads.iter(app.world()).next().unwrap();
	let records = app.world().get::<ThreadRecords>(thread).unwrap();
	records
		.actors()
		.values()
		.map(|actor| actor.kind())
		.collect()
}

/// Count of reduced agents: an `ActorRef` carrying a model streamer.
fn agent_count(app: &mut App) -> usize {
	let mut agents = app
		.world_mut()
		.query_filtered::<Entity, (With<ActorRef>, With<O11sStreamer>)>();
	agents.iter(app.world()).count()
}

/// Count of tools the reduced agents hold, each agent's the routes of the
/// `<Router>` beneath it.
fn tool_count(app: &mut App) -> usize {
	app.world_mut()
		.with_state::<(Query<Entity, With<ActorRef>>, ThreadQuery), _>(
			|(agents, query)| {
				agents
					.iter()
					.map(|agent| query.tools(agent).unwrap().len())
					.sum()
			},
		)
}

#[beet::test]
async fn chat_scene_reduces() {
	let mut app = reduce(include_str!("../examples/thread/chat.bsx")).await;
	assert_crate_check_passed(&mut app);
	assert_served_shape(&mut app);
	record_counts(&mut app).xpect_eq((3, 1));
	agent_count(&mut app).xpect_eq(1);
	// the `kind="System"` / `kind="User"` attributes resolve, rather than
	// silently defaulting to `Agent`
	let kinds = actor_kinds(&mut app);
	kinds.contains(&ActorKind::User).xpect_true();
	kinds.contains(&ActorKind::System).xpect_true();
}

#[beet::test]
async fn multi_agent_scene_reduces() {
	let mut app =
		reduce(include_str!("../examples/thread/multi_agent.bsx")).await;
	assert_crate_check_passed(&mut app);
	assert_served_shape(&mut app);
	record_counts(&mut app).xpect_eq((3, 1));
	// two differently-instructed agents reduce side by side
	agent_count(&mut app).xpect_eq(2);
}

#[beet::test]
async fn tool_call_scene_reduces() {
	let mut app =
		reduce(include_str!("../examples/thread/tool_call.bsx")).await;
	assert_crate_check_passed(&mut app);
	assert_served_shape(&mut app);
	record_counts(&mut app).xpect_eq((2, 1));
	agent_count(&mut app).xpect_eq(1);
	// the inline `<AgentChoiceAction/>` reduced to a routed tool
	tool_count(&mut app).xpect_eq(1);
}

#[beet::test]
async fn self_evolving_scene_reduces() {
	let mut app =
		reduce(include_str!("../examples/thread/self_evolving.bsx")).await;
	assert_crate_check_passed(&mut app);
	assert_served_shape(&mut app);
	record_counts(&mut app).xpect_eq((2, 1));
	agent_count(&mut app).xpect_eq(1);
	// the `<StoreToolset/>` reduced to the five blob tools
	tool_count(&mut app).xpect_eq(5);
}

#[beet::test]
async fn coding_agent_scene_reduces() {
	let mut app =
		reduce(include_str!("../examples/thread/coding_agent.bsx")).await;
	assert_crate_check_passed(&mut app);
	assert_served_shape(&mut app);
	record_counts(&mut app).xpect_eq((2, 1));
	agent_count(&mut app).xpect_eq(1);
	tool_count(&mut app).xpect_eq(5);
}

#[beet::test]
async fn persistent_chat_scene_reduces() {
	let mut app =
		reduce(include_str!("../examples/thread/persistent_chat.bsx")).await;
	enter_mounted(&mut app).await;
	assert_crate_check_passed(&mut app);
	assert_served_shape(&mut app);
	record_counts(&mut app).xpect_eq((3, 1));
	agent_count(&mut app).xpect_eq(1);
	let kinds = actor_kinds(&mut app);
	kinds.contains(&ActorKind::User).xpect_true();
	kinds.contains(&ActorKind::System).xpect_true();
}

/// Enter every mounted thread one after the other, as one `RunThread` would,
/// over one store at each, so the second window finds the thread the first
/// created.
#[beet::test]
async fn group_chat_scene_opens_two_windows() {
	let mut app =
		reduce(include_str!("../examples/thread/group_chat.bsx")).await;
	declare_temp_tables(&mut app);
	let holder = app
		.world_mut()
		.query_filtered_once::<Entity, With<RunThread>>()[0];
	app.world_mut()
		.entity_mut(holder)
		.call::<Request, Outcome>(Request::default())
		.await
		.unwrap()
		.xpect_eq(Outcome::PASS);
	assert_crate_check_passed(&mut app);
	agent_count(&mut app).xpect_eq(0);
	let windows = app.world_mut().query_once::<(Entity, &Thread)>();
	windows.len().xpect_eq(2);
	windows[0].1.id().xpect_eq(windows[1].1.id());
	let windows = windows
		.into_iter()
		.map(|(entity, _)| entity)
		.collect::<HashSet<_>>();
	// a view and a composer each, the composer posting as its own person
	app.world_mut()
		.query_filtered_once::<&OfThread, With<ThreadView>>()
		.into_iter()
		.map(OfThread::thread)
		.collect::<HashSet<_>>()
		.xpect_eq(windows);
	app.world_mut()
		.query_once::<&CreatePostForm>()
		.into_iter()
		.filter_map(|form| form.actor)
		.collect::<HashSet<_>>()
		.len()
		.xpect_eq(2);
}

#[beet::test]
async fn by_the_way_scene_reduces() {
	let mut app =
		reduce(include_str!("../examples/thread/by_the_way.bsx")).await;
	assert_crate_check_passed(&mut app);
	app.world_mut()
		.query_filtered_once::<Entity, (With<ActorRef>, With<InterruptOnPost>)>(
		)
		.len()
		.xpect_eq(1);
	record_counts(&mut app).xpect_eq((3, 1));
	agent_count(&mut app).xpect_eq(1);
	// the `<RouteTool/>` resolved the surrounding `survey` route
	tool_count(&mut app).xpect_eq(1);
}

/// The function example's route names the template beside it, whose agent's
/// one tool is a route.
#[beet::test]
async fn function_scene_declares_its_route() {
	let mut app = reduce(include_str!("../examples/thread/function.bsx")).await;
	assert_crate_check_passed(&mut app);
	let routes = app.world_mut().query_once::<&ThreadRoute>();
	routes.len().xpect_eq(1);
	match &routes[0].source {
		TemplateSource::Document(document) => {
			document.as_str().xpect_eq("templates/ReviewThread.bsx")
		}
		TemplateSource::Template(name) => panic!("a template, `{name}`"),
	};
	routes[0].message.is_some().xpect_true();
	let mut app = reduce(include_str!(
		"../examples/thread/templates/ReviewThread.bsx"
	))
	.await;
	agent_count(&mut app).xpect_eq(0);
	app.world_mut()
		.query_once::<&RouteToolCall>()
		.len()
		.xpect_eq(1);
}
