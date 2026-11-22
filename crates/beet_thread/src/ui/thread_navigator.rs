//! The owner's navigator: the threads of a workspace as a tree by lineage.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::Declaration;
use beet_ui::prelude::Focusable;
use beet_ui::prelude::PointerUp;
use beet_ui::prelude::inline_class;
use beet_ui::prelude::material::colors;
use beet_ui::prelude::style;

// ═══════════════════════════════════════════════════════════════════════
// ThreadNavigator
// ═══════════════════════════════════════════════════════════════════════

/// The threads kept beneath `path`, as a tree by lineage: a thread under the
/// thread whose post spawned it ([`Thread::spawned_by`]), a branch under the
/// thread it branches from, marked `⑂` ([`Thread::branch_of`]), each live
/// thread marked, and one awaiting its owner ([`AwaitingOwner`]) marked so.
///
/// The tree is read from records, every thread store beneath `path` in the
/// store above the navigator ([`BlobThreadStore::discover`]), joined by
/// [`ThreadId`] with the live thread entities of the world, so a thread a
/// session is running shows before its records are written.
///
/// Selecting a row binds the [`ThreadView`] and [`CreatePostForm`] beside the
/// navigator, beneath its parent, to the thread through [`OfThread`]: its
/// live entity, or for a stored thread none is running, a view-only entity
/// holding its records with no behavior ([`ViewOnlyThread`]), whose composer
/// is hidden, since nothing would read a post sent there. With `follow` the
/// navigator selects whichever thread is awaiting the owner, so the owner
/// answers each ask where it arises.
///
/// ```bsx
/// <div>
///     <ThreadNavigator path="packages/acme/assets/threads" follow=true/>
///     <ThreadView/>
///     <CreatePostForm/>
/// </div>
/// ```
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Document, NavigatorState)]
#[component(on_add = ThreadNavigator::on_add)]
pub struct ThreadNavigator {
	/// The directory holding the thread stores, in the store above the
	/// navigator.
	pub path: RelPath,
	/// Select whichever thread is awaiting the owner.
	pub follow: bool,
}

impl Default for ThreadNavigator {
	fn default() -> Self {
		Self {
			path: RelPath::new("threads"),
			follow: false,
		}
	}
}

/// A navigator's records and selection, view state never persisted.
#[derive(Default, Component)]
pub struct NavigatorState {
	/// Every stored thread found, with the store keeping it.
	records: Vec<(Thread, ThreadStore)>,
	/// Whether the records need reading again, ie a live session ended and
	/// wrote its records.
	stale: bool,
	/// The selected thread.
	selected: Option<ThreadId>,
	/// The thread the items beside the navigator are bound to.
	bound: Option<ThreadId>,
	/// The view-only entity spawned for a stored thread with no live one.
	view_only: Option<Entity>,
}

impl NavigatorState {
	/// The selected thread.
	pub fn selected(&self) -> Option<ThreadId> { self.selected }
	/// Select `thread`, binding the items beside the navigator to it.
	pub fn select(&mut self, thread: ThreadId) { self.selected = Some(thread); }
}

/// Marks a thread entity a navigator spawned to show a stored thread no
/// session is running: its records, no behavior, never live.
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
pub struct ViewOnlyThread;

/// Marks a navigator row, carrying the thread it stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
#[component(on_add = hook_ext::observe(NavigatorRow::select_on_activate))]
pub struct NavigatorRow {
	/// The thread this row stands for.
	pub thread: ThreadId,
}

impl ThreadNavigator {
	/// Attach the keyed row list and mark the records unread.
	fn on_add(mut world: DeferredWorld, cx: HookContext) {
		if let Some(mut state) = world.get_mut::<NavigatorState>(cx.entity) {
			state.stale = true;
		}
		world.commands().entity(cx.entity).insert(rsx! {
			<div {(
				FieldRef::new("rows"),
				ReactiveChildren::keyed(Self::row_key, Self::row),
				navigator_style(),
			)}/>
		});
	}

	/// The tree's rows in display order: depth first, each thread under its
	/// parent by lineage, roots and siblings oldest first.
	pub fn rows(
		records: &[Thread],
		live: &HashMap<ThreadId, bool>,
		selected: Option<ThreadId>,
	) -> Vec<NavigatorRowData> {
		let ids = records.iter().map(Thread::id).collect::<HashSet<_>>();
		// a thread's parent: the thread it branches from, else the one whose
		// post spawned it, when that thread is in the tree
		let parent = |thread: &Thread| {
			thread
				.branch_of()
				.or(thread.spawned_by())
				.map(PostRef::thread)
				.filter(|parent| ids.contains(parent))
		};
		let mut children = HashMap::<Option<ThreadId>, Vec<&Thread>>::default();
		for thread in records {
			children.entry(parent(thread)).or_default().push(thread);
		}
		for siblings in children.values_mut() {
			siblings.sort_by_key(|thread| (thread.created(), thread.id()));
		}
		let mut rows = Vec::with_capacity(records.len());
		let mut stack = children
			.remove(&None)
			.unwrap_or_default()
			.into_iter()
			.rev()
			.map(|thread| (thread, None::<(Vec<bool>, bool)>))
			.collect::<Vec<_>>();
		while let Some((thread, place)) = stack.pop() {
			let guides = place
				.as_ref()
				.map(|(ancestors, has_next)| {
					ancestors
						.iter()
						.map(|open| match open {
							true => "│\u{a0}",
							false => "\u{a0}\u{a0}",
						})
						.chain([match has_next {
							true => "├\u{a0}",
							false => "└\u{a0}",
						}])
						.collect::<String>()
				})
				.unwrap_or_default();
			rows.push(NavigatorRowData::new(
				thread,
				guides,
				live.get(&thread.id()).copied(),
				selected == Some(thread.id()),
			));
			let lineage = place
				.map(|(mut ancestors, has_next)| {
					ancestors.push(has_next);
					ancestors
				})
				.unwrap_or_default();
			let kids = children.remove(&Some(thread.id())).unwrap_or_default();
			let last = kids.len().saturating_sub(1);
			for (index, kid) in kids.into_iter().enumerate().rev() {
				stack.push((kid, Some((lineage.clone(), index != last))));
			}
		}
		rows
	}

	/// A row's key: its thread and everything it shows, so a row whose
	/// markers change rebuilds and no other does.
	fn row_key(item: &Value) -> String { item.to_string() }

	/// Build one row: its guides and label, focusable so a click or Enter
	/// selects it.
	fn row(_index: usize, item: &Value) -> OnSpawn {
		let text = item
			.get("text")
			.and_then(|text| text.as_str().ok())
			.unwrap_or_default()
			.to_string();
		let thread = item
			.get("id")
			.and_then(|id| id.as_str().ok())
			.and_then(|id| ThreadId::parse(id).ok());
		OnSpawn::new(move |entity| {
			entity.insert(rsx! {
				<div {(Focusable, row_style())} tabindex="0">{text}</div>
			});
			if let Some(thread) = thread {
				entity.insert(NavigatorRow { thread });
			}
			Ok(())
		})
	}
}

/// One row of the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigatorRowData {
	/// The thread the row stands for.
	pub thread: ThreadId,
	/// The row as painted: the selection marker, the guides drawing its place
	/// in the tree, its label and its markers.
	pub text: String,
}

impl NavigatorRowData {
	/// The row for `thread`, `live` holding whether it awaits the owner when
	/// a session is running it.
	fn new(
		thread: &Thread,
		guides: String,
		live: Option<bool>,
		selected: bool,
	) -> Self {
		let label = thread.name();
		let text = format!(
			"{}{guides}{}{label}{}",
			if selected { "> " } else { "  " },
			if thread.branch_of().is_some() {
				"⑂ "
			} else {
				""
			},
			match live {
				Some(true) => " [awaiting]",
				Some(false) => " [live]",
				None => "",
			}
		);
		Self {
			thread: thread.id(),
			text,
		}
	}

	/// The row as a document item.
	fn value(&self) -> Value {
		Value::Map(Map::new([
			("id", Value::new(self.thread.to_string())),
			("text", Value::new(self.text.clone())),
		]))
	}
}

impl NavigatorRow {
	/// Observer: activating a row (a click, or Enter while focused) selects
	/// its thread on the navigator above it.
	fn select_on_activate(
		ev: On<PointerUp>,
		rows: Query<&NavigatorRow>,
		ancestors: Query<&ChildOf>,
		mut states: Query<&mut NavigatorState>,
	) -> Result {
		// the event bubbles; act only at the row itself
		let target = ev.event_target();
		let Ok(row) = rows.get(target) else {
			return Ok(());
		};
		let Some(navigator) = ancestors
			.iter_ancestors(target)
			.find(|ancestor| states.contains(*ancestor))
		else {
			return Ok(());
		};
		states.get_mut(navigator)?.select(row.thread);
		Ok(())
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Systems
// ═══════════════════════════════════════════════════════════════════════

/// The live thread entities of the world: entered by a session, holding
/// their store, never a navigator's view-only copy.
type LiveThreads<'w, 's> = Query<
	'w,
	's,
	(Entity, &'static Thread, Has<AwaitingOwner>),
	(With<ThreadStore>, Without<ViewOnlyThread>),
>;

/// Read the records of every navigator whose records are stale, and mark
/// them stale again whenever a live session ends, since it has just written
/// its records.
pub(crate) fn read_navigator_records(
	async_commands: AsyncCommands,
	mut ended: RemovedComponents<ThreadStore>,
	mut navigators: Query<(Entity, &ThreadNavigator, &mut NavigatorState)>,
) {
	let ended = ended.read().count() > 0;
	for (entity, navigator, mut state) in navigators.iter_mut() {
		if !(state.stale || ended) {
			continue;
		}
		state.stale = false;
		let dir = navigator.path.clone();
		async_commands.run(async move |world: AsyncWorld| -> Result {
			let navigator = world.entity(entity);
			let store = navigator
				.with_state::<AncestorQuery<&BlobStore>, _>(|entity, query| {
					query.get(entity).cloned()
				})
				.await?
				.map_err(|_| {
					bevyhow!("no store above the navigator to find `{dir}` in")
				})?;
			let mut records = Vec::new();
			for found in BlobThreadStore::discover(&store, &dir).await? {
				let store = ThreadStore::new(found);
				for thread in store.threads().await? {
					records.push((thread, store.clone()));
				}
			}
			navigator
				.with(move |mut entity: EntityWorldMut| {
					if let Some(mut state) = entity.get_mut::<NavigatorState>()
					{
						state.records = records;
					}
				})
				.await?;
			Ok(())
		});
	}
}

/// Select whichever thread awaits the owner, for every navigator following.
pub(crate) fn follow_awaiting_owner(
	live: LiveThreads,
	mut navigators: Query<(&ThreadNavigator, &mut NavigatorState)>,
) {
	let Some(awaiting) = live
		.iter()
		.filter(|(_, _, awaiting)| *awaiting)
		.map(|(_, thread, _)| thread.id())
		.max()
	else {
		return;
	};
	for (navigator, mut state) in navigators.iter_mut() {
		let current_awaits = state.selected.is_some_and(|selected| {
			live.iter().any(|(_, thread, awaiting)| {
				awaiting && thread.id() == selected
			})
		});
		if navigator.follow && !current_awaits {
			state.selected = Some(awaiting);
		}
	}
}

/// Project every navigator's tree into its [`Document`]: its records and the
/// live threads, joined by id.
pub(crate) fn project_navigators(
	live: LiveThreads,
	mut navigators: Query<
		(&NavigatorState, &mut Document),
		With<ThreadNavigator>,
	>,
) {
	let markers = live
		.iter()
		.map(|(_, thread, awaiting)| (thread.id(), awaiting))
		.collect::<HashMap<_, _>>();
	for (state, mut document) in navigators.iter_mut() {
		let mut records = state
			.records
			.iter()
			.map(|(thread, _)| thread.clone())
			.collect::<Vec<_>>();
		for (_, thread, _) in live.iter() {
			match records.iter_mut().find(|record| record.id() == thread.id()) {
				Some(record) => *record = thread.clone(),
				None => records.push(thread.clone()),
			}
		}
		let rows = ThreadNavigator::rows(&records, &markers, state.selected)
			.iter()
			.map(NavigatorRowData::value)
			.collect::<Vec<_>>();
		let value = Value::Map(Map::new([("rows", Value::List(rows))]));
		if document.0 != value {
			document.0 = value;
		}
	}
}

/// Bind the items beside each navigator to its selected thread: its live
/// entity, else a view-only entity loaded from its store, hiding the
/// composer of a thread no session reads.
pub(crate) fn bind_navigator_selection(
	mut commands: Commands,
	async_commands: AsyncCommands,
	live: LiveThreads,
	parents: Query<&ChildOf>,
	children: Query<&Children>,
	views: Query<(), With<ThreadView>>,
	forms: Query<(), With<CreatePostForm>>,
	attributes: Query<&Attributes>,
	keys: Query<&Attribute>,
	mut navigators: Query<(Entity, &mut NavigatorState), With<ThreadNavigator>>,
) {
	for (navigator, mut state) in navigators.iter_mut() {
		let Some(selected) = state.selected else {
			continue;
		};
		if state.bound == Some(selected) {
			continue;
		}
		let scope = parents
			.get(navigator)
			.map(ChildOf::parent)
			.unwrap_or(navigator);
		let items = children
			.iter_descendants(scope)
			.filter(|entity| views.contains(*entity) || forms.contains(*entity))
			.collect::<Vec<_>>();
		let live_entity = live
			.iter()
			.find(|(_, thread, _)| thread.id() == selected)
			.map(|(entity, ..)| entity);
		let target = match live_entity {
			Some(entity) => entity,
			None => {
				let Some((record, store)) = state
					.records
					.iter()
					.find(|(thread, _)| thread.id() == selected)
					.cloned()
				else {
					// selected before its records are read: bind once they are
					continue;
				};
				let entity = commands.spawn(ViewOnlyThread).id();
				async_commands.run(async move |world: AsyncWorld| -> Result {
					let records = store.load_records(&record).await?;
					let synced = records.posts().iter().map(Post::id).collect();
					world
						.entity(entity)
						.insert((
							record,
							records,
							store,
							SyncedPosts::new(synced),
						))
						.await?;
					Ok(())
				});
				entity
			}
		};
		if let Some(previous) = state.view_only.take() {
			commands.entity(previous).try_despawn();
		}
		if live_entity.is_none() {
			state.view_only = Some(target);
		}
		for item in &items {
			commands.entity(*item).insert(OfThread(target));
		}
		// a view-only thread's composer is hidden, a live one's shown
		for form in items.iter().filter(|item| forms.contains(**item)) {
			let hidden = attributes.get(*form).ok().and_then(|attributes| {
				attributes.iter().find(|attribute| {
					keys.get(*attribute)
						.is_ok_and(|key| key.as_str() == "hidden")
				})
			});
			match (live_entity.is_some(), hidden) {
				(true, Some(hidden)) => {
					commands.entity(hidden).despawn();
				}
				(false, None) => {
					commands.spawn((
						AttributeOf::new(*form),
						Attribute::new("hidden"),
					));
				}
				_ => {}
			}
		}
		state.bound = Some(selected);
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Style
// ═══════════════════════════════════════════════════════════════════════

/// The tree: a column of rows.
fn navigator_style() -> OnSpawn {
	inline_class![
		(style::common_props::DisplayProp, style::Display::Flex),
		(
			style::common_props::FlexDirectionProp,
			style::Direction::Vertical
		),
		(style::common_props::OverflowYProp, style::Overflow::Auto),
	]
}

/// A row: one line, never wrapped, on the surface tint.
fn row_style() -> OnSpawn {
	inline_class![
		(style::common_props::WhiteSpaceProp, style::WhiteSpace::Pre),
		Declaration::token(
			style::common_props::ForegroundColor,
			colors::OnSurface
		),
	]
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::ui::test_ext;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use bevy::math::UVec2;

	/// A thread named as its row reads.
	fn thread(name: &str) -> Thread { Thread::new(name) }

	/// A completed text post on `thread`.
	fn post(thread: &Thread, actor: &Actor, text: &str) -> Post {
		AgentPost::new_text(
			actor.id(),
			thread.id(),
			text,
			PostStatus::Completed,
		)
	}

	/// A workspace's thread stores under `threads/`, related as a loop run
	/// leaves them: the loop's two decisions each spawning a unit thread, one
	/// of which a branch continues. Answers the store and the threads.
	async fn workspace() -> (BlobStore, Vec<Thread>) {
		let store = BlobStore::temp();
		let actor = Actor::agent();
		let records = |role: &str| {
			ThreadStore::new(BlobThreadStore::new(
				store.with_subdir(RelPath::new("threads").join(role)),
			))
		};
		let loop_thread = thread("LoopThread journal=loop");
		let grade = post(&loop_thread, &actor, "grade product: unmet evals");
		let write = post(&loop_thread, &actor, "write market: missing");
		let grader = thread("GraderThread")
			.with_spawned_by(PostRef::new(loop_thread.id(), grade.id()));
		let clerk = thread("ClerkThread document=market")
			.with_spawned_by(PostRef::new(loop_thread.id(), write.id()));
		let answer = post(&clerk, &actor, "wrote the market");
		let branch = thread("ClerkThread document=pricing")
			.with_branch_of(PostRef::new(clerk.id(), answer.id()));
		for (role, record, posts) in [
			("loop", &loop_thread, vec![grade, write]),
			("grader", &grader, vec![]),
			("clerk", &clerk, vec![answer]),
			("clerk", &branch, vec![]),
		] {
			let records = records(role);
			records.insert_thread(record.clone()).await.unwrap();
			records.insert_actor(actor.clone()).await.unwrap();
			records.insert_posts(posts).await.unwrap();
		}
		(store, vec![loop_thread, grader, clerk, branch])
	}

	/// A navigator in follow mode over `store`'s `threads/`, beside a view
	/// and a composer, painted on a host: the app, the host and the page.
	fn navigator(store: &BlobStore) -> (App, Entity, Entity) {
		let mut app = test_ext::render_app();
		let (host, page) = test_ext::mount_page(
			&mut app,
			UVec2::new(60, 12),
			Snippet::from_bundle(rsx! {
				<div>
					<div {ThreadNavigator{path:RelPath::new("threads"), follow:true}}/>
					<ThreadView/>
					<CreatePostForm/>
				</div>
			}),
		);
		app.world_mut().entity_mut(page).insert(store.clone());
		(app, host, page)
	}

	/// Pump until the records are read and the tree painted.
	async fn settle(app: &mut App) {
		for _ in 0..4 {
			test_ext::settle(app);
			async_ext::yield_now().await;
		}
	}

	/// The navigator's state.
	fn state(app: &mut App) -> &NavigatorState {
		let navigator = app
			.world_mut()
			.query_filtered::<Entity, With<ThreadNavigator>>()
			.single(app.world())
			.unwrap();
		app.world().get::<NavigatorState>(navigator).unwrap()
	}

	/// The thread every item beside the navigator is bound to.
	fn bound(app: &mut App) -> Vec<Entity> {
		app.world_mut()
			.query_filtered::<&OfThread, Or<(With<ThreadView>, With<CreatePostForm>)>>(
			)
			.iter(app.world())
			.map(OfThread::thread)
			.collect()
	}

	/// The tree nests each unit under the loop decision that spawned it, and
	/// a branch, marked, under the thread it branches from.
	#[beet_core::test]
	async fn renders_the_lineage_as_a_tree() {
		let (store, _) = workspace().await;
		let (mut app, host, _) = navigator(&store);
		settle(&mut app).await;
		test_ext::frame(&app, host).xpect_snapshot();
	}

	/// Follow mode focuses the live thread awaiting the owner, binding the
	/// view and the composer beside the navigator to its entity, and a
	/// stored thread selected after binds a view-only entity, its composer
	/// hidden.
	#[beet_core::test]
	async fn follows_the_thread_awaiting_the_owner() {
		let (store, threads) = workspace().await;
		let (mut app, host, _) = navigator(&store);
		settle(&mut app).await;
		let live = app
			.world_mut()
			.spawn((
				threads[2].clone(),
				ThreadRecords::default(),
				ThreadStore::default(),
				AwaitingOwner,
			))
			.id();
		settle(&mut app).await;
		state(&mut app).selected().xpect_eq(Some(threads[2].id()));
		bound(&mut app).xpect_eq(vec![live, live]);
		test_ext::frame(&app, host)
			.xpect_contains("> ")
			.xpect_contains("document=market [awaiting]");

		// the owner answers, and selects the grader no session runs
		app.world_mut().entity_mut(live).remove::<AwaitingOwner>();
		let navigator = app
			.world_mut()
			.query_filtered::<Entity, With<ThreadNavigator>>()
			.single(app.world())
			.unwrap();
		app.world_mut()
			.get_mut::<NavigatorState>(navigator)
			.unwrap()
			.select(threads[1].id());
		settle(&mut app).await;
		let view_only = bound(&mut app)[0];
		app.world().get::<ViewOnlyThread>(view_only).xpect_some();
		app.world()
			.get::<Thread>(view_only)
			.unwrap()
			.id()
			.xpect_eq(threads[1].id());
		let form = app
			.world_mut()
			.query_filtered::<Entity, With<CreatePostForm>>()
			.single(app.world())
			.unwrap();
		app.world_mut()
			.query::<(&Attribute, &AttributeOf)>()
			.iter(app.world())
			.any(|(attribute, of)| {
				attribute.as_str() == "hidden" && of.get() == form
			})
			.xpect_true();
	}
}
