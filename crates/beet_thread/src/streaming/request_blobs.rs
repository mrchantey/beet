//! The files a request carries, see [`RequestBlobs`].
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::exports::bytes::Bytes;

/// What one request does with each file its posts hold, decided before its
/// mapper maps so neither mapper reads a store: a request is built in two
/// steps, the records answering its posts and then the bytes they reference
/// read.
///
/// The newest files its agent's [`KeepBlobs`] keeps are carried, counted by
/// file whatever post holds them, their bytes held in memory on the post or
/// read from the window's [`ThreadStore`]; any older one is listed. A carried
/// file that cannot be read is unavailable, a warning and never a failed
/// turn. The default carries every file a post holds in memory.
#[derive(Debug, Default, Clone)]
pub struct RequestBlobs {
	/// The bytes read from the store, by content.
	read: HashMap<Cid, Bytes>,
	/// The files listed rather than carried.
	listed: HashSet<FileKey>,
}

/// One file as a request sends it ([`RequestBlobs::files`]).
#[derive(Debug, Clone)]
pub enum RequestFile {
	/// Its bytes, carried.
	Carried(PostFile, Bytes),
	/// Older than the context keeps: its listing line
	/// ([`PostFile::listing`]).
	Listed(PostFile),
	/// Carried, but its bytes cannot be read: [`RequestBlobs::UNAVAILABLE`].
	Unavailable(PostFile),
}

impl RequestFile {
	/// The text a reader that is not sent the bytes reads: a listed file's
	/// listing, an unavailable one's [`RequestBlobs::UNAVAILABLE`], else the
	/// file's line.
	pub fn text(&self) -> String {
		match self {
			Self::Carried(file, _) => file.line(),
			Self::Listed(file) => file.listing(),
			Self::Unavailable(_) => RequestBlobs::UNAVAILABLE.to_string(),
		}
	}

	/// The file and its bytes, if carried.
	pub fn carried(&self) -> Option<(&PostFile, &Bytes)> {
		match self {
			Self::Carried(file, bytes) => Some((file, bytes)),
			_ => None,
		}
	}
}

impl RequestBlobs {
	/// What a model and a view read in place of a file whose bytes are gone.
	pub const UNAVAILABLE: &'static str = "[file unavailable]";

	/// The files of `shown` an agent keeping `keep` lists, and the
	/// references of those it carries that no post holds in memory, to read.
	/// A deleted post's file is in neither, its body gone from what is shown.
	pub fn plan(
		shown: &ShownPosts,
		keep: Option<usize>,
	) -> (Self, Vec<BlobRef>) {
		let carried = keep.map(|keep| PostFile::newest(shown.files(), keep));
		let is_carried = |file: &PostFile| {
			carried
				.as_ref()
				.is_none_or(|carried| carried.contains(&file.key()))
		};
		let listed = shown
			.files()
			.filter(|file| !is_carried(file))
			.map(|file| file.key())
			.collect();
		let mut missing = Vec::<BlobRef>::new();
		for file in shown.files().filter(is_carried) {
			let held = shown
				.post(file.post())
				.is_some_and(|post| post.file_bytes(file.index()).is_some());
			if !held && !missing.iter().any(|blob| blob.cid == file.blob().cid)
			{
				missing.push(file.blob().clone());
			}
		}
		let blobs = Self {
			read: default(),
			listed,
		};
		(blobs, missing)
	}

	/// Read each of `missing` from the store of the window `thread`, a blob
	/// that cannot be read warned and left out.
	pub async fn read(
		mut self,
		thread: &AsyncEntity,
		missing: Vec<BlobRef>,
	) -> Result<Self> {
		if missing.is_empty() {
			return self.xok();
		}
		let store = thread
			.with_state::<Query<&ThreadStore>, _>(|entity, stores| {
				stores.get(entity).ok().cloned()
			})
			.await?;
		for blob in missing {
			let read = match &store {
				Some(store) => store.read_blob(&blob).await,
				None => Err(bevyhow!("the thread mounts no store")),
			};
			match read {
				Ok(bytes) => {
					self.read.insert(blob.cid, bytes);
				}
				Err(err) => {
					warn!("the file {} is unavailable: {err}", blob.cid)
				}
			}
		}
		self.xok()
	}

	/// The files of the context of the agent at `caller`, by its
	/// [`KeepBlobs`], with the bytes it carries that no post holds in memory
	/// read: a post a compaction folded away carries nothing.
	#[cfg(feature = "action")]
	pub async fn for_agent(caller: &AsyncEntity) -> Result<Self> {
		let (thread, blobs, missing) = caller
			.with_state::<(ThreadQuery, Query<&KeepBlobs>), _>(
				|entity, (query, keep)| -> Result<_> {
					let (thread, _, records) =
						query.thread_and_records(entity)?;
					let reader = query.actor_id(entity)?;
					let keep = keep.get(entity).ok().map(|keep| keep.count);
					let shown = records.shown(..);
					let bounds = records.context_bounds(&shown, &reader);
					let context = shown
						.within(|post| ThreadRecords::in_context(post, bounds));
					let (blobs, missing) = Self::plan(&context, keep);
					(thread, blobs, missing).xok()
				},
			)
			.await??;
		blobs.read(&caller.world().entity(thread), missing).await
	}

	/// The text a tool's output reads as where its files' bytes are not
	/// sent: its answer's text, then each file's text
	/// ([`RequestFile::text`]), a line each.
	pub fn output_text(&self, output: &Post) -> String {
		output
			.field("text")
			.filter(|text| !text.is_empty())
			.map(str::to_string)
			.into_iter()
			.chain(self.files(output).iter().map(RequestFile::text))
			.collect::<Vec<_>>()
			.join("\n")
	}

	/// How a request sends each file `post` holds, in order.
	pub fn files(&self, post: &Post) -> Vec<RequestFile> {
		post.files()
			.into_iter()
			.map(|file| {
				if self.listed.contains(&file.key()) {
					return RequestFile::Listed(file);
				}
				match post
					.file_bytes(file.index())
					.cloned()
					.or_else(|| self.read.get(&file.blob().cid).cloned())
				{
					Some(bytes) => RequestFile::Carried(file, bytes),
					None => RequestFile::Unavailable(file),
				}
			})
			.collect()
	}
}

#[cfg(all(test, feature = "agent"))]
mod test {
	use crate::prelude::*;
	use crate::streaming::request_test;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_scene::prelude::*;

	/// Bytes standing in for a png, its media type following its name.
	const CAT: &[u8] = b"cat";
	const DOG: &[u8] = b"dog";

	/// A person and an agent carrying one file, looking through `read-blob`
	/// at a store holding `cat.png`: the app, the window and the agent.
	async fn looker() -> (App, Entity, Entity) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_types::<ThreadPlugin>();
		let source = r#"
<Thread {Sequence}>
	<CreateActor name="Ann" kind="User"/>
	<CreateActor name="Looker" kind="Agent" {(MockPostStreamer{script:[Call{tool:"read-blob", arguments:{path:"cat.png"}}]}, KeepBlobs{count:1})}>
		<Router><StoreToolset read_only=true/></Router>
	</CreateActor>
</Thread>
"#;
		let store = BlobStore::temp();
		store
			.insert(&fwd_path!("cat.png"), CAT.to_vec())
			.await
			.unwrap();
		let window = BsxParser::spawn(app.world_mut(), source).unwrap();
		app.world_mut().entity_mut(window).insert(store);
		app.update();
		ThreadRecords::reduce_now(app.world_mut());
		let agent = request_test::actor_entity(
			&mut app,
			&actor_id!("looker"),
		);
		(app, window, agent)
	}

	/// Ann posts `bytes` as a png.
	fn post_png(app: &mut App, window: Entity, bytes: &[u8]) {
		let mut window = app.world_mut().entity_mut(window);
		let thread = window.get::<Thread>().unwrap().id();
		window.get_mut::<ThreadRecords>().unwrap().upsert_post(
			AgentPost::new_media(
				actor_id!("ann"),
				thread,
				MediaType::Png,
				bytes.to_vec(),
				Some("photo".into()),
				PostStatus::Completed,
			),
		);
	}

	/// The data url both protocols carry png `bytes` as.
	fn data_url(bytes: &[u8]) -> String {
		format!(
			"data:image/png;base64,{}",
			base64::Engine::encode(&base64::prelude::BASE64_STANDARD, bytes)
		)
	}

	/// How often `needle` is in `request`'s json.
	fn count(request: &Value, needle: &str) -> usize {
		request.to_json_string().matches(needle).count()
	}

	/// A person's image reaches each protocol as an image, its bytes
	/// encoded by the protocol's mapper.
	#[beet_core::test]
	async fn an_image_reaches_each_protocol() {
		let (mut app, window, agent) = looker().await;
		post_png(&mut app, window, CAT);
		let [responses, completions] =
			request_test::requests(&mut app, agent).await;
		let item = &responses["input"][0];
		item["role"].as_str().unwrap().xpect_eq("user");
		item["content"][0]["type"]
			.as_str()
			.unwrap()
			.xpect_eq("input_image");
		item["content"][0]["image_url"]
			.as_str()
			.unwrap()
			.xpect_eq(data_url(CAT));
		let message = &completions["messages"][0];
		message["role"].as_str().unwrap().xpect_eq("user");
		message["content"][0]["image_url"]["url"]
			.as_str()
			.unwrap()
			.xpect_eq(data_url(CAT));
	}

	/// A tool answering an image reaches open responses as its output's
	/// content parts, and completions, whose tool message is text alone, as
	/// that text, the file's line, then a user message holding it.
	#[beet_core::test]
	async fn a_tool_answering_an_image_reaches_each_protocol() {
		let (mut app, _, agent) = looker().await;
		app.world_mut()
			.entity_mut(agent)
			.call::<(), Outcome>(())
			.await
			.unwrap();
		let [responses, completions] =
			request_test::requests(&mut app, agent).await;
		let line = "[file: read-blob.png, image/png, 3 B]";

		let output = &responses["input"][1];
		output["type"]
			.as_str()
			.unwrap()
			.xpect_eq("function_call_output");
		output["output"][0]["text"].as_str().unwrap().xpect_eq(line);
		output["output"][1]["type"]
			.as_str()
			.unwrap()
			.xpect_eq("input_image");
		output["output"][1]["image_url"]
			.as_str()
			.unwrap()
			.xpect_eq(data_url(CAT));

		let messages = &completions["messages"];
		messages.as_list().unwrap().len().xpect_eq(3);
		messages[1]["role"].as_str().unwrap().xpect_eq("tool");
		messages[1]["content"].as_str().unwrap().xpect_eq(line);
		messages[2]["role"].as_str().unwrap().xpect_eq("user");
		messages[2]["content"][0]["image_url"]["url"]
			.as_str()
			.unwrap()
			.xpect_eq(data_url(CAT));
	}

	/// Past its `KeepBlobs`, an older file is listed by its post, and once a
	/// tool reads the same bytes back its answer is the newest file, carried
	/// again.
	#[beet_core::test]
	async fn a_listed_file_read_by_a_tool_is_carried_again() {
		let (mut app, window, agent) = looker().await;
		post_png(&mut app, window, CAT);
		post_png(&mut app, window, DOG);
		let cat =
			app.world().get::<ThreadRecords>(window).unwrap().posts()[0].id();
		let listing =
			format!("[file: photo.png, image/png, 3 B] file 1 in post {cat}");
		for request in request_test::requests(&mut app, agent).await {
			count(&request, &listing).xpect_eq(1);
			count(&request, &data_url(CAT)).xpect_eq(0);
			count(&request, &data_url(DOG)).xpect_eq(1);
		}
		app.world_mut()
			.entity_mut(agent)
			.call::<(), Outcome>(())
			.await
			.unwrap();
		for request in request_test::requests(&mut app, agent).await {
			count(&request, &listing).xpect_eq(1);
			count(&request, " in post ").xpect_eq(2);
			count(&request, &data_url(CAT)).xpect_eq(1);
			count(&request, &data_url(DOG)).xpect_eq(0);
		}
	}
}
