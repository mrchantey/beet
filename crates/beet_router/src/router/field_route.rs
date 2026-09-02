//! The markup front-end for a document-field route: a path, a field and the
//! operation to serve over it.
//!
//! The Rust twin is `(FieldRef, route::exchange(path, action))`, which
//! `examples/todo/todo.rs` writes out by hand. Doing the same from markup needs
//! this template because the piece that makes a typed action reachable over
//! `Request -> Response`, the [`ExchangeOverload`], is deliberately not
//! [`Reflect`] and so can never ride a component spread.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;

/// The document operation a [`FieldRoute`] serves, chosen by its `verb` prop.
///
/// Each variant names the [`common_actions`](beet_ui::prelude) action the route
/// dispatches to, and with it the request shape: a `--body` (or JSON `POST`
/// body) deserializes into the action's input, and its output is negotiated back
/// as the response.
///
/// A write answers with the field's new value rather than an empty `null`, so
/// `curl -d .. /sign` verifies itself; [`RemoveAt`](Self::RemoveAt) answers with
/// the item it dropped, the one thing a follow-up read cannot recover.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum FieldVerb {
	/// [`ReadDocField`]: answer with the whole field. No input.
	#[default]
	Read,
	/// [`ReadAtDocField`]: answer with one list item. Input is the index.
	ReadAt,
	/// [`PushDocField`]: append to the list. Input is the item.
	Push,
	/// [`SetDocField`]: overwrite the field. Input is the new value.
	Set,
	/// [`SetAtDocField`]: replace one list item. Input is `[index, value]`.
	SetAt,
	/// [`RemoveAtDocField`]: drop one list item, answering with it. Input is the
	/// index.
	RemoveAt,
}

/// A route over one [`Document`] field, the CRUD counterpart of the scripted
/// [`ScriptRoute`] and the markup generalization of `examples/todo/todo.rs`:
///
/// ```bsx
/// <FieldRoute path="sign" field="entries" verb="Push" list/>
/// <FieldRoute path="list" field="entries" verb="Read" list/>
/// ```
///
/// The field resolves like any other [`FieldRef`], against the nearest ancestor
/// [`Document`], so sibling routes over the same `field` share one list and the
/// pages binding `@doc:entries` see every write.
///
/// Every verb reads and writes that document *directly*, through the
/// `*DocField` actions rather than their self-bound twins: a route is a
/// request/response boundary with no frames in between, so a `POST` must be
/// visible to the very next `GET` and not one settle later.
///
/// Several routes over one field are several entities, because an entity holds
/// at most one action: that is why this is a route front-end and not a bundle of
/// verbs on a single entity.
#[template]
pub fn FieldRoute(
	/// The route path, eg `sign`.
	#[prop(into)]
	path: String,
	/// The document field, dotted for a nested one, eg `entries` or `user.name`.
	#[prop(into)]
	field: String,
	/// Which document operation to serve, spelled as the [`FieldVerb`] variant,
	/// ie `verb="Push"`. An unknown name fails the load listing the valid ones.
	verb: FieldVerb,
	/// Seed a missing field with `[]` rather than `null`, so a read of an
	/// untouched list answers `[]` and not `null`. The twin of todo.rs's
	/// `FieldRef::new("todos").with_init(Value::List(Vec::new()))`.
	list: bool,
) -> Result<impl Bundle> {
	if path.is_empty() || field.is_empty() {
		bevybail!(
			"`<FieldRoute>` needs both a `path` and a `field`, ie `<FieldRoute path=\"list\" field=\"entries\" verb=\"Read\"/>`"
		);
	}
	let mut field_ref = FieldRef::new(FieldPath::new(field.split('.')));
	if list {
		field_ref = field_ref.with_init(Value::List(Vec::new()));
	}
	// each arm is a different bundle type, so the route is erased behind the
	// shared `OnSpawn` insert seam rather than boxed per arm.
	let route = match verb {
		FieldVerb::Read => {
			OnSpawn::insert(route::exchange(&path, ReadDocField))
		}
		FieldVerb::ReadAt => {
			OnSpawn::insert(route::exchange(&path, ReadAtDocField))
		}
		FieldVerb::Push => {
			OnSpawn::insert(route::exchange(&path, PushDocField))
		}
		FieldVerb::Set => OnSpawn::insert(route::exchange(&path, SetDocField)),
		FieldVerb::SetAt => {
			OnSpawn::insert(route::exchange(&path, SetAtDocField))
		}
		FieldVerb::RemoveAt => {
			OnSpawn::insert(route::exchange(&path, RemoveAtDocField))
		}
	};
	Ok((field_ref, route))
}

#[cfg(test)]
// the routes exchange typed bodies, which need a serialization format;
// run with `cargo test -p beet_router --features json`.
#[cfg(feature = "json")]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// Settle the freshly built tree so the seeded fields reach their document
	/// before the first dispatch.
	fn settle(world: &mut World) { world.update_local(); }

	/// Spawn `markup` as a subtree of a fresh router, returning the router.
	///
	/// Built from a BSX string rather than `rsx!` so the props take the real
	/// string-attribute coercion path a `main.bsx` uses, and so the routes land
	/// under an intermediate entity, as a template build leaves them.
	fn router_with(markup: &str) -> (World, Entity) {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world.spawn(Router::with_defaults()).flush();
		let nodes =
			BsxNode::parse_document(markup, &BsxParseConfig::bsx()).unwrap();
		world
			.spawn(ChildOf(root))
			.insert_template(BsxTemplate::container(
				nodes,
				BsxTemplateRegistry::default(),
			))
			.unwrap();
		world.flush();
		settle(&mut world);
		(world, root)
	}

	/// A guestbook whose two routes address one `entries` list: the property the
	/// whole front-end rests on, since each route is its own entity with its own
	/// [`FieldRef`] and they only agree by resolving the same document.
	fn guestbook() -> (World, Entity) {
		router_with(
			r#"<FieldRoute path="sign" field="entries" verb="Push" list/>
			   <FieldRoute path="list" field="entries" verb="Read" list/>"#,
		)
	}

	/// Sign the guestbook, returning the response body, ie the new list.
	async fn sign(world: &mut World, root: Entity, name: &str) -> String {
		let res = world
			.entity_mut(root)
			.exchange(
				Request::with_json(
					"sign",
					&value!({ "name": name, "message": "hi" }),
				)
				.unwrap(),
			)
			.await;
		res.status().is_success().xpect_true();
		res.unwrap_str().await
	}

	/// The body of a `GET list`.
	async fn list(world: &mut World, root: Entity) -> String {
		world
			.entity_mut(root)
			.exchange(Request::get("list"))
			.await
			.unwrap_str()
			.await
	}

	#[beet_core::test]
	async fn reads_an_untouched_list_as_empty() {
		let (mut world, root) = guestbook();
		list(&mut world, root).await.xpect_eq("[]".to_string());
	}

	#[beet_core::test]
	async fn push_reaches_the_sibling_read() {
		let (mut world, root) = guestbook();
		sign(&mut world, root, "Ada").await;
		settle(&mut world);
		list(&mut world, root)
			.await
			.xpect_contains("Ada")
			.xpect_contains("hi");
	}

	/// The staleness this whole front-end turns on: `curl sign; curl sign; curl
	/// list` back to back, every request inside one frame, so nothing the bidi
	/// sync does between frames can help. A route is a request/response boundary
	/// with no frames to spare, which is why the verbs read and write the
	/// document directly instead of a locally mirrored [`Value`].
	#[beet_core::test]
	async fn signs_land_without_a_settle() {
		let (mut world, root) = guestbook();
		// each write answers with the new list, so the sign verifies itself
		sign(&mut world, root, "Ada").await.xpect_contains("Ada");
		sign(&mut world, root, "Bob")
			.await
			.xpect_contains("Ada")
			.xpect_contains("Bob");
		list(&mut world, root)
			.await
			.xpect_contains("Ada")
			.xpect_contains("Bob");
	}

	/// Signs *with* a frame between them, the case
	/// [`signs_land_without_a_settle`] cannot cover: a settle runs the bidi sync
	/// both ways, and a mirrored [`Value`] echoing back over the document would
	/// undo the signature the previous request appended.
	#[beet_core::test]
	async fn successive_signs_all_land() {
		let (mut world, root) = guestbook();
		for name in ["Ada", "Bob", "Cy"] {
			sign(&mut world, root, name).await;
			// a frame between requests, as a live server has
			settle(&mut world);
		}
		list(&mut world, root)
			.await
			.xpect_contains("Ada")
			.xpect_contains("Bob")
			.xpect_contains("Cy");
	}

	/// The index-addressed verbs, ie the `read`/`update`/`delete` routes
	/// `examples/todo/todo.rs` writes by hand.
	#[beet_core::test]
	async fn index_verbs_address_one_list() {
		let (mut world, root) = router_with(
			r#"<FieldRoute path="create" field="todos" verb="Push" list/>
			   <FieldRoute path="read"   field="todos" verb="ReadAt" list/>
			   <FieldRoute path="update" field="todos" verb="SetAt" list/>
			   <FieldRoute path="delete" field="todos" verb="RemoveAt" list/>
			   <FieldRoute path="list"   field="todos" verb="Read" list/>"#,
		);
		for description in ["wash", "dry"] {
			world
				.entity_mut(root)
				.exchange(
					Request::with_json(
						"create",
						&value!({ "description": description }),
					)
					.unwrap(),
				)
				.await
				.status()
				.is_success()
				.xpect_true();
			settle(&mut world);
		}
		world
			.entity_mut(root)
			.exchange(Request::with_json("read", &1usize).unwrap())
			.await
			.unwrap_str()
			.await
			.xpect_contains("dry");
		world
			.entity_mut(root)
			.exchange(
				Request::with_json(
					"update",
					&(0usize, value!({ "description": "fold" })),
				)
				.unwrap(),
			)
			.await
			.status()
			.is_success()
			.xpect_true();
		settle(&mut world);
		world
			.entity_mut(root)
			.exchange(Request::with_json("delete", &1usize).unwrap())
			.await
			.unwrap_str()
			.await
			.xpect_contains("dry");
		settle(&mut world);
		world
			.entity_mut(root)
			.exchange(Request::get("list"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("fold")
			.xnot()
			.xpect_contains("dry");
	}

	/// A `<FieldRoute>` with no `field` is an author error, surfaced through the
	/// build channel as a [`TemplateError`] rather than a silent no-op route.
	#[beet_core::test]
	fn missing_field_surfaces_error() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn_template(
				rsx! { <FieldRoute path="list" verb=FieldVerb::Read/> },
			)
			.unwrap()
			.id();
		world
			.entity(root)
			.get::<TemplateError>()
			.unwrap()
			.error
			.to_string()
			.xpect_contains("needs both a `path` and a `field`");
	}
}
