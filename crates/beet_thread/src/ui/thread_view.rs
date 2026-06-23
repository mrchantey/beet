//! The reactive transcript widget: a [`ThreadWindow`] projected into a
//! [`Document`] and rendered as a scrollable, keyed list of posts.

use crate::prelude::*;
// `Table::id()` on a `Post` (via `PostView`'s deref); the `beet_ui` glob below
// otherwise shadows the prelude re-export of this trait.
use crate::table::Table;
use beet_core::prelude::*;
// `ScrollPosition` is beet_ui's renderer-agnostic type; pin it explicitly so it wins
// over bevy's same-named ui type (reached via `beet_core::prelude` under
// `bevy_default`). The rest of beet_ui's prelude arrives via `crate::prelude`.
use beet_ui::prelude::ScrollPosition;

// ═══════════════════════════════════════════════════════════════════════
// ThreadView: the reactive chat widget
// ═══════════════════════════════════════════════════════════════════════

/// A reactive view of a thread, rendering its [`ThreadWindow`] as a scrollable
/// list of posts. Carries its own [`Document`] (seeded empty, filled by
/// [`project_window_to_document`]), so the inner [`ReactiveChildren`] and the
/// per-row [`FieldRef`]s resolve against it via `DocumentPath::Ancestor`.
///
/// Host-agnostic content, not a host: spawn it under any render surface (a
/// charcell terminal host, a web page) and bind it to its thread with an
/// [`OfThread`] relationship. A marker, so the bound thread lives in the
/// relationship, not a stored field. From markup the two spread together onto one
/// entity, so the same view serves a local terminal and a per-connection server
/// surface alike:
///
/// ```rsx
/// <div bx:ref="thread" {Thread} {Sequence}>..</div>
/// <div {(ThreadView, OfThread($thread))}/>
/// ```
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Document)]
#[component(on_add = thread_view_on_add)]
pub struct ThreadView;

impl ThreadView {
	/// A view bound to `thread`. Its reactive content is attached in `on_add`, so
	/// the bundle works both as a direct spawn and as a markup spread.
	pub fn new(thread: Entity) -> impl Bundle { (Self, OfThread(thread)) }
}

/// Attach the reactive post list when a [`ThreadView`] is added: a scroll
/// container whose children are one row per `posts` item, keyed by post id so
/// appends reuse settled rows.
fn thread_view_on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).insert(rsx! {
		<div {(
			ThreadScroll,
			ScrollPosition::default(),
			FieldRef::new("posts"),
			ReactiveChildren::keyed(post_key, post_row),
		)}/>
	});
}

/// Marks a [`ThreadView`]'s scroll container, so [`follow_thread_scroll`] can
/// pin it to the latest post on append.
#[derive(Debug, Clone, Copy, Component, Reflect)]
#[reflect(Component)]
pub struct ThreadScroll;

/// Stable reconciliation key for a post item: its `id` field (a uuid string), so
/// reconciliation reuses a row across appends and in-progress body growth.
fn post_key(item: &Value) -> String {
	item.get("id")
		.and_then(|id| id.as_str().ok())
		.unwrap_or_default()
		.to_string()
}

/// Build one post row: the author name, then the post body bound through a
/// [`FieldRef`] so streamed text re-syncs in place. The row's terminating scope
/// is `posts[index]`, so `text` resolves to `posts[index].text`.
fn post_row(_index: usize, item: &Value) -> OnSpawn {
	let author = item
		.get("author")
		.and_then(|author| author.as_str().ok())
		.unwrap_or_default()
		.to_string();
	OnSpawn::insert(rsx! {
		<div>
			<span>{author}": "</span>
			<span>{(Value::default(), FieldRef::new("text"))}</span>
		</div>
	})
}

// ═══════════════════════════════════════════════════════════════════════
// Projection: ThreadWindow -> Document
// ═══════════════════════════════════════════════════════════════════════

/// Project every changed [`ThreadWindow`] into the [`Document`] of each
/// [`ThreadView`] watching its thread, and (per the contract) into a [`Document`]
/// on the thread entity itself.
///
/// The document holds every post (display intent or not, so reasoning and tool
/// traffic remain inspectable) as `{ "posts": [{ id, author, text }, ..] }`.
/// Keyed reconciliation downstream means a grown in-progress body updates a row
/// rather than rebuilding it, so streaming flows through the bound [`Value`].
pub fn project_window_to_document(
	mut commands: Commands,
	windows: Query<
		(Entity, &ThreadWindow, Option<&ThreadItems>),
		Changed<ThreadWindow>,
	>,
	views: Query<(), With<ThreadView>>,
	mut documents: Query<&mut Document>,
) -> Result {
	for (thread_entity, window, items) in windows.iter() {
		let value = project_window(window);
		// the contract's thread-side document, inserted if absent
		set_document(
			&mut commands,
			&mut documents,
			thread_entity,
			value.clone(),
		);
		// every view item of this thread renders against its own co-located document,
		// reached by traversing the thread's `ThreadItems` instead of scanning views.
		items
			.into_iter()
			.flat_map(|items| items.iter())
			.filter(|item| views.contains(*item))
			.for_each(|view_entity| {
				set_document(
					&mut commands,
					&mut documents,
					view_entity,
					value.clone(),
				);
			});
	}
	Ok(())
}

/// Follow-on-append: when a [`ThreadView`]'s document changes (a post was added
/// or grew), pin its [`ThreadScroll`] container to the bottom by parking the
/// offset past the end. `clamp_scroll_positions` re-clamps it to the true max
/// next frame, against the freshly laid-out content.
pub fn follow_thread_scroll(
	views: Query<Entity, (With<ThreadView>, Changed<Document>)>,
	children: Query<&Children>,
	mut scrolls: Query<&mut ScrollPosition, With<ThreadScroll>>,
) {
	for view in views.iter() {
		for descendant in children.iter_descendants(view) {
			if let Ok(mut scroll) = scrolls.get_mut(descendant) {
				scroll.offset.y = i32::MAX;
			}
		}
	}
}

/// Build the document value for a window: a `posts` list of `{ id, author, text }`.
fn project_window(window: &ThreadWindow) -> Value {
	let posts = window
		.post_views()
		.map(|view| {
			Value::Map(
				[
					("id".into(), Value::new(view.post.id().to_string())),
					("author".into(), Value::new(view.actor.name())),
					("text".into(), Value::new(view.post.to_string())),
				]
				.into_iter()
				.collect(),
			)
		})
		.collect::<Vec<_>>();
	Value::Map([("posts".into(), Value::List(posts))].into_iter().collect())
}

/// Update `entity`'s [`Document`] in place, or insert one if it has none yet.
fn set_document(
	commands: &mut Commands,
	documents: &mut Query<&mut Document>,
	entity: Entity,
	value: Value,
) {
	match documents.get_mut(entity) {
		Ok(mut document) => document.0 = value,
		Err(_) => {
			commands.entity(entity).insert(Document::new(value));
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The declarative binding the scenes rely on: an `{(ThreadView, OfThread($thread))}`
	/// spread resolves its `$thread` reference to the `bx:ref="thread"` Thread entity
	/// (the relationship machinery remaps it), so the view renders the thread a sibling
	/// subtree declares, with no Rust glue. The thread then names the view through its
	/// `ThreadItems`.
	#[beet_core::test]
	fn thread_view_binds_to_referenced_thread() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<ThreadPlugin>()
			.init_plugin::<ThreadUiPlugin>();
		let source = r#"
<div>
	<div bx:ref="thread" {Thread} {Sequence}/>
	<span {(ThreadView, OfThread($thread))}/>
</div>
"#;
		BsxTemplate::parse_entry(app.world(), source)
			.unwrap()
			.spawn(app.world_mut())
			.unwrap();
		app.world_mut().flush();

		let thread = app
			.world_mut()
			.query_filtered::<Entity, With<Thread>>()
			.single(app.world())
			.unwrap();
		let (view, of_thread) = app
			.world_mut()
			.query_filtered::<(Entity, &OfThread), With<ThreadView>>()
			.single(app.world())
			.unwrap();
		// the `$thread` placeholder remapped to the real Thread entity ...
		of_thread.thread().xpect_eq(thread);
		// ... and the thread names the view back through its `ThreadItems`.
		app.world()
			.entity(thread)
			.get::<ThreadItems>()
			.unwrap()
			.iter()
			.any(|item| item == view)
			.xpect_true();
	}
}
