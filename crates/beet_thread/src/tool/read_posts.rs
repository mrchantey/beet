//! `<ReadPosts/>`: an agent reading its thread's posts by id.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// An agent's tool reading a run of its thread's posts by id, `read-posts`,
/// each file a post holds answered with its bytes.
///
/// ```bsx
/// <CreateActor name="Coach" kind="Agent" {ModelStreamer{provider:OpenAi}}>
/// 	<Router><ReadPosts/></Router>
/// </CreateActor>
/// ```
///
/// What it reads is decided by where it sits, never by a flag an author
/// could set (the model page, [`crate::table`], "Everything and shown"): in
/// a conversation's agent it reads that conversation, shown; in a
/// compaction thread, whose build [`Compacting`] marks with the window it
/// compacts, it reads that conversation, everything, a deleted post's body
/// and an edit's original included, so the compactor finds for itself what
/// its message does not say. It never reads a compaction.
///
/// It reads up to [`ReadPosts::MOST`] posts of the thread's history a call,
/// from the post `from` on, across a branch's lineage: from the window's
/// records where they hold the run, else from its store. An answer holding
/// files is one `multipart/mixed` body (RFC 2046,
/// [`Multipart`](beet_net::prelude::Multipart)), the listing first then each
/// file, which an agent's tool call splits into its output's files.
#[template]
pub fn ReadPosts() -> impl Bundle {
	route::exchange(ReadPosts::PATH, ReadPostsCall)
}

impl ReadPosts {
	/// The tool's route path.
	pub const PATH: &'static str = "read-posts";
	/// The most posts one call reads.
	pub const MOST: usize = 50;
	/// The posts a call reads when it names no count.
	pub const DEFAULT: usize = 20;
}

/// The arguments of a [`ReadPosts`] call.
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
pub struct ReadPostsProps {
	/// The id of the first post to read.
	pub from: PostId,
	/// How many posts to read, 20 by default and 50 at most.
	#[serde(default)]
	pub count: Option<usize>,
}

/// The action of a [`ReadPosts`]: the run of posts from `from`, as a
/// listing and, for the files among them, their bytes.
#[action]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn ReadPostsCall(
	cx: ActionContext<ReadPostsProps>,
) -> Result<Response> {
	let ReadPostsProps { from, count } = cx.input;
	let count = count
		.unwrap_or(ReadPosts::DEFAULT)
		.clamp(1, ReadPosts::MOST);
	// the conversation a compaction reads everything of, else the agent's own
	let (window, everything) = cx
		.caller
		.with_state::<(Query<&ChildOf>, Query<&Compacting>, ThreadRecordsQuery), _>(
			|entity, (ancestors, compacting, query)| -> Result<_> {
				match ancestors
					.iter_ancestors_inclusive(entity)
					.find_map(|ancestor| compacting.get(ancestor).ok())
				{
					Some(compacting) => (compacting.0, true).xok(),
					None => (query.thread_entity(entity)?, false).xok(),
				}
			},
		)
		.await??;
	let world = cx.world();
	let (held, store, thread) = world
		.with(move |world: &mut World| -> Result<_> {
			let records =
				world.get::<ThreadRecords>(window).ok_or_else(|| {
					bevyhow!("the thread {window} holds no records")
				})?;
			let held = (records.post(from).is_some()
				|| records.run_start().is_none_or(|start| from >= start))
			.then(|| records.read_from(from, count));
			let store = world.get::<ThreadStore>(window).cloned();
			let thread = world.get::<Thread>(window).cloned();
			(held, store, thread).xok()
		})
		.await?;
	let run = match (held, store, thread) {
		(Some(run), _, _) => run,
		(None, Some(store), Some(thread)) => {
			store.read_from(&thread, from, count).await?
		}
		_ => ThreadRecords::new(),
	};
	let posts = match everything {
		true => run.posts().to_vec(),
		false => run.shown(..).to_vec(),
	};
	let lines = posts
		.iter()
		.filter_map(|post| {
			let author = run
				.actor(post.author())
				.map_or(post.author().as_str(), |actor| actor.name());
			Compaction::post_line(post, author, everything)
		})
		.collect::<Vec<_>>();
	let listing = match lines.is_empty() {
		true => format!("no post from {from}"),
		false => lines.join("\n"),
	};
	// each file of what was read, its bytes held or read from the store
	let mut multipart =
		Multipart::default().with_part(MediaBytes::new_text(&listing).into());
	for post in posts
		.iter()
		.filter(|post| post.kind() != &PostKind::Compaction)
	{
		for file in post.files() {
			let bytes = match post.file_bytes(file.index()) {
				Some(bytes) => Some(bytes.clone()),
				None => match world
					.with(move |world: &mut World| {
						world.get::<ThreadStore>(window).cloned()
					})
					.await
				{
					Some(store) => store.read_blob(file.blob()).await.ok(),
					None => None,
				},
			};
			if let Some(bytes) = bytes {
				multipart.push(
					MultipartPart::new(MediaBytes::new(
						file.media_type(),
						bytes,
					))
					.with_filename(file.filename()),
				);
			}
		}
	}
	match multipart.parts().len() {
		1 => Response::ok_body(listing, MediaType::Text),
		_ => multipart.into_response(),
	}
	.xok()
}

impl ThreadRecords {
	/// Up to `count` posts of these records from `from` on, with their
	/// authors and metadata, as records of their own.
	pub(crate) fn read_from(&self, from: PostId, count: usize) -> Self {
		let start = self.posts().partition_point(|post| post.id() < from);
		let mut run = ThreadRecords::new();
		for post in self.posts()[start..].iter().take(count) {
			if let Ok(actor) = self.actor(post.author()) {
				run.insert_actor(actor.clone());
			}
			if let Some(meta) = self.meta(post.id()) {
				run.set_meta(meta.clone());
			}
			run.upsert_post(Post::clone(post));
		}
		run
	}
}

#[cfg(all(test, feature = "agent"))]
mod test {
	use crate::prelude::*;
	use crate::types::compact::test::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_router::prelude::*;
	use beet_scene::prelude::*;

	/// A conversation whose coach reads posts, keeping `keep` files.
	const READER: &str = r#"<Thread {(Sequence, MountThreadStore{path:"chat"})}>
	<CreateActor name="Ann" kind="User"/>
	<CreateActor name="Coach" kind="Agent" {MockPostStreamer}>
		<Router><ReadPosts/></Router>
	</CreateActor>
</Thread>"#;

	/// The text of `router`'s answer to reading from `from`.
	async fn read(app: &mut App, router: Entity, from: PostId) -> String {
		let request = Request::get(ReadPosts::PATH)
			.with_body(format!(r#"{{"from":"{from}"}}"#))
			.with_header::<header::ContentType>(MediaType::Json);
		let response =
			app.world_mut().entity_mut(router).exchange(request).await;
		match Multipart::from_response(response).await.unwrap() {
			Ok(multipart) => {
				String::from_utf8_lossy(multipart.parts()[0].media().bytes())
					.to_string()
			}
			Err(response) => response.text().await.unwrap(),
		}
	}

	/// The router beneath `actor`.
	fn router_of(chat: &mut Chat, actor: &ActorId) -> Entity {
		let actor = chat.actor(actor);
		chat.app
			.world_mut()
			.query_filtered::<(Entity, &ChildOf), With<Router>>()
			.iter(chat.app.world())
			.find(|(_, parent)| parent.parent() == actor)
			.unwrap()
			.0
	}

	/// The conversation's agent reads what is shown, a compaction thread's
	/// reads everything, and neither reads a compaction.
	#[beet_core::test]
	async fn reads_by_where_it_sits() {
		let mut chat = Chat::of(READER).await;
		let original = chat.say(&ANN, "ORIGINAL-FACT");
		let deleted = chat.say(&ANN, "DELETED-FACT");
		chat.say(&ANN, "KEPT-FACT");
		chat.edit(&ANN, original.id(), "NEW-FACT").await;
		chat.delete(&ANN, deleted.id()).await;
		chat.compacted(&COACH, deleted.id(), "COMPACTION-TEXT");
		let shown = router_of(&mut chat, &COACH);
		read(&mut chat.app, shown, original.id())
			.await
			.as_str()
			.xpect_contains("NEW-FACT")
			.xpect_contains("KEPT-FACT")
			.xpect_contains("(deleted)")
			.xnot()
			.xpect_contains("ORIGINAL-FACT")
			.xnot()
			.xpect_contains("DELETED-FACT")
			.xnot()
			.xpect_contains("COMPACTION-TEXT");
		// a compaction thread's, marked with the window it compacts
		let everything = BsxParser::spawn(
			chat.app.world_mut(),
			"<Router><ReadPosts/></Router>",
		)
		.unwrap();
		chat.app
			.world_mut()
			.entity_mut(everything)
			.insert(Compacting(chat.window));
		chat.app.update();
		read(&mut chat.app, everything, original.id())
			.await
			.as_str()
			.xpect_contains("NEW-FACT")
			.xpect_contains("which said: ORIGINAL-FACT")
			.xpect_contains("(deleted) DELETED-FACT")
			.xnot()
			.xpect_contains("COMPACTION-TEXT");
	}

	/// A call reading several posts' files answers one multipart body, its
	/// output holding each file: open responses sends them as the output's
	/// parts and completions as one user message after the tool message,
	/// and `KeepBlobs` counts files, listing an older one by post and index.
	#[beet_core::test]
	async fn several_files_reach_each_protocol_as_parts() {
		for keep in [3, 1] {
			let mut chat = Chat::of(READER).await;
			let coach = chat.actor(&COACH);
			let mut photos = Vec::new();
			for bytes in [b"cat", b"dog"] {
				let mut window = chat.app.world_mut().entity_mut(chat.window);
				let thread = window.get::<Thread>().unwrap().id();
				let post = AgentPost::new_media(
					ANN,
					thread,
					MediaType::Png,
					bytes.to_vec(),
					Some("photo".into()),
					PostStatus::Completed,
				);
				photos.push(post.id());
				window.get_mut::<ThreadRecords>().unwrap().upsert_post(post);
			}
			chat.app.world_mut().entity_mut(coach).insert((
				MockPostStreamer::with_script([MockTurn::call(
					ReadPosts::PATH,
					value!({ "from": photos[0].to_string() }),
				)
				.unwrap()]),
				KeepBlobs::new(keep),
			));
			chat.turn(&COACH).await;
			let output = chat
				.records()
				.posts()
				.iter()
				.find(|post| post.kind() == &PostKind::ToolOutput)
				.unwrap()
				.clone();
			output.files().len().xpect_eq(2);
			output
				.field("text")
				.unwrap()
				.xpect_contains(&photos[1].to_string());
			let [responses, completions] = chat.requests(&COACH).await;
			let images = |text: &str| text.matches("data:image/png").count();
			match keep {
				// both of the output's files and Ann's newest carried
				3 => {
					images(&responses).xpect_eq(3);
					images(&completions).xpect_eq(3);
					let completions = Value::parse_json(&completions).unwrap();
					let messages = completions["messages"].as_list().unwrap();
					let tool = messages
						.iter()
						.position(|message| {
							message["role"]
								.as_str()
								.is_ok_and(|role| role == "tool")
						})
						.unwrap();
					messages[tool + 1]["content"]
						.as_list()
						.unwrap()
						.len()
						.xpect_eq(2);
				}
				// the newest alone, the rest listed by post and index
				_ => {
					images(&responses).xpect_eq(1);
					images(&completions).xpect_eq(1);
					for request in [&responses, &completions] {
						request
							.as_str()
							.xpect_contains(&format!(
								"file 1 in post {}",
								output.id()
							))
							.xpect_contains(&format!(
								"file 1 in post {}",
								photos[0]
							));
					}
				}
			}
		}
	}
}
