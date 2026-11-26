//! The files a request carries, see [`RequestBlobs`].
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::exports::bytes::Bytes;

/// What one request does with each file its posts hold, decided before its
/// mapper maps so neither mapper reads a store: a request is built in two
/// steps, the records answering its posts and then the bytes they reference
/// read.
///
/// The newest files its agent's [`KeepBlobs`] keeps are carried, their
/// bytes held in memory on the post or read from the window's
/// [`ThreadStore`]; any older one is listed. A carried file that cannot be
/// read is unavailable, a warning and never a failed turn. The default
/// carries every file a post holds in memory.
#[derive(Debug, Default, Clone)]
pub struct RequestBlobs {
	/// The bytes read from the store, by content.
	read: HashMap<Cid, Bytes>,
	/// The posts whose files are listed rather than carried.
	listed: HashSet<PostId>,
}

/// One file as a request sends it ([`RequestBlobs::file`]).
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
}

impl RequestBlobs {
	/// What a model and a view read in place of a file whose bytes are gone.
	pub const UNAVAILABLE: &'static str = "[file unavailable]";

	/// The files of `records` an agent keeping `keep` lists, and the
	/// references of those it carries that no post holds in memory, to read.
	pub fn plan(
		records: &ThreadRecords,
		keep: Option<usize>,
	) -> (Self, Vec<BlobRef>) {
		let carried = keep.map(|keep| records.carried_files(keep));
		let is_carried = |file: &PostFile| {
			carried
				.as_ref()
				.is_none_or(|carried| carried.contains(&file.post()))
		};
		let listed = records
			.files()
			.filter(|file| !is_carried(file))
			.map(|file| file.post())
			.collect();
		let mut missing = Vec::<BlobRef>::new();
		for file in records.files().filter(is_carried) {
			let held = records
				.post(file.post())
				.is_some_and(|post| post.bytes().is_some());
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

	/// The files of the records of the window of the agent at `caller`, by
	/// its [`KeepBlobs`], with the bytes it carries that no post holds in
	/// memory read.
	#[cfg(feature = "action")]
	pub async fn for_agent(caller: &AsyncEntity) -> Result<Self> {
		let (thread, blobs, missing) = caller
			.with_state::<(ThreadQuery, Query<&KeepBlobs>), _>(
				|entity, (query, keep)| -> Result<_> {
					let (thread, _, records) =
						query.thread_and_records(entity)?;
					let keep = keep.get(entity).ok().map(|keep| keep.count);
					let (blobs, missing) = Self::plan(records, keep);
					(thread, blobs, missing).xok()
				},
			)
			.await??;
		blobs.read(&caller.world().entity(thread), missing).await
	}

	/// The text a tool's output reads as where its bytes are not sent: a
	/// file's ([`RequestFile::text`]), else the output.
	pub fn output_text(&self, output: &Post) -> String {
		match self.file(output) {
			Some(file) => file.text(),
			None => output.field("text").unwrap_or_default().to_string(),
		}
	}

	/// How a request sends the file `post` holds, `None` for a post holding
	/// none.
	pub fn file(&self, post: &Post) -> Option<RequestFile> {
		let file = post.file()?;
		if self.listed.contains(&post.id()) {
			return RequestFile::Listed(file).xsome();
		}
		match post
			.bytes()
			.cloned()
			.or_else(|| self.read.get(&file.blob().cid).cloned())
		{
			Some(bytes) => RequestFile::Carried(file, bytes),
			None => RequestFile::Unavailable(file),
		}
		.xsome()
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
			.insert(&RelPath::new("cat.png"), CAT.to_vec())
			.await
			.unwrap();
		let window = BsxParser::spawn(app.world_mut(), source).unwrap();
		app.world_mut().entity_mut(window).insert(store);
		app.update();
		ThreadRecords::reduce_now(app.world_mut());
		let agent = request_test::actor_entity(
			&mut app,
			&ActorId::from_static("looker"),
		);
		(app, window, agent)
	}

	/// Ann posts `bytes` as a png.
	fn post_png(app: &mut App, window: Entity, bytes: &[u8]) {
		let mut window = app.world_mut().entity_mut(window);
		let thread = window.get::<Thread>().unwrap().id();
		window.get_mut::<ThreadRecords>().unwrap().upsert_post(
			AgentPost::new_media(
				ActorId::from_static("ann"),
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
			format!("[file: photo.png, image/png, 3 B] in post {cat}");
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
