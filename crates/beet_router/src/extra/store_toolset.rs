//! The standard blob-store agent toolset.
//!
//! This composes [`route::exchange`] with `beet_net`'s blob-store actions, so it
//! lives with the other `extra` router pieces (eg [`ServeBlobs`]) rather than
//! in a downstream crate. An agent crate re-exports it for its scenes.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Equip an agent with the standard blob-store toolset: list, read, write,
/// edit, and remove against the nearest ancestor [`BlobStore`]. Each entry is a
/// routed [`route::exchange`], so the agent both sees the tool's schema and can
/// dispatch the call. It writes through [`WriteText`] rather than
/// [`WriteBlob`], whose byte array a model spells badly.
///
/// A `#[template]`, so it nests under an agent's [`Router`] in markup, the url
/// space its tools are the routes of, ie
/// `<CreateActor name="Coder" kind="Agent" {ModelStreamer{provider:OpenAi}}><Router><StoreToolset/></Router></CreateActor>`,
/// with a [`BlobStore`] mounted on an ancestor (eg the thread's behavior root).
/// `<StoreToolset read_only=true/>` mounts the two reading tools alone, for an
/// agent that may look at a store but never change it: a call of any other
/// is its router's not-found.
#[template]
pub fn StoreToolset(
	/// Mount only `list-blobs` and `read-blob`.
	read_only: bool,
) -> impl Bundle {
	OnSpawn::new(move |entity| {
		entity.with_children(|parent| {
			parent.spawn(route::exchange("list-blobs", ListBlobs));
			parent.spawn(route::exchange("read-blob", ReadBlob));
			if !read_only {
				parent.spawn(route::exchange("write-text", WriteText));
				parent.spawn(route::exchange("edit-text", EditText));
				parent.spawn(route::exchange("remove-blob", RemoveBlob));
			}
		});
	})
}

// `<StoreToolset/>` equipping the routed blob tools (and the `ToolDefinition`
// an agent derives from each) is covered downstream where the tool-definition
// observer lives: `tests/thread_scenes.rs` (`coding_agent`/`self_evolving` reduce
// to a `tool_count` of 5) and beet_thread's `read_only_toolset_cannot_write`.

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A typed-input tool reads its JSON body as its input, never as params,
	/// so a strict server passes its fields: an agent's `write-text` call
	/// lands.
	#[cfg(feature = "json")]
	#[beet_core::test]
	async fn typed_calls_pass_a_strict_server() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let store = BlobStore::temp();
		let router = BsxTemplate::parse_entry(
			&world,
			"<Router><StoreToolset/></Router>",
		)
		.unwrap()
		.spawn(&mut world)
		.unwrap();
		world
			.spawn((CliServer::default(), store.clone()))
			.add_child(router);
		world.flush();
		world
			.entity_mut(router)
			.exchange(
				Request::get("write-text")
					.with_body(r#"{"path":"a.txt","text":"hi"}"#)
					.with_content_type(MediaType::Json),
			)
			.await
			.into_result()
			.await
			.unwrap();
		store
			.get(&RelPath::new("a.txt"))
			.await
			.unwrap()
			.to_vec()
			.xpect_eq(b"hi".to_vec());
	}

	/// Two toolsets under one router are two routes on each path of one url
	/// space, which the tree build refuses, naming both entities.
	#[beet_core::test]
	fn two_toolsets_in_one_url_space_conflict() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		// the observer-driven rebuild raises too, so log rather than panic
		world.insert_resource(bevy::ecs::error::FallbackErrorHandler(
			bevy::ecs::error::warn,
		));
		let router = BsxTemplate::parse_entry(
			&world,
			"<Router><StoreToolset/><StoreToolset/></Router>",
		)
		.unwrap()
		.spawn(&mut world)
		.unwrap();
		world.flush();
		let err = world
			.run_system_cached_with::<_, Result, _, _>(
				RouteTree::rebuild,
				router,
			)
			.unwrap()
			.unwrap_err()
			.to_string()
			.xpect_contains("Duplicate route");
		// the build refuses the first duplicate it meets, so find its pair by path
		let routes = world
			.query_filtered::<(Entity, &PathPattern), With<ActionMeta>>()
			.iter(&world)
			.filter(|(_, path)| {
				err.contains(&format!("`/{}`", path.annotated_path()))
			})
			.map(|(entity, _)| entity)
			.collect::<Vec<_>>();
		routes.len().xpect_eq(2);
		err.xpect_contains(&routes[0].to_string())
			.xpect_contains(&routes[1].to_string());
	}
}
