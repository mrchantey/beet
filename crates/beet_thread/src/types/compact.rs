//! Compacting an agent's context, see [`Compact`].
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;
use std::sync::Arc;

/// On an agent: its context compacts once it grows past `at` tokens, its
/// manners as [`InterruptOnPost`] is. The agent never summarises itself:
/// another thread does, built from `template` for one compaction, run as a
/// function and despawned, its model named there and never assumed to be the
/// conversation's (the model page, [`crate::table`], "Contexts and
/// compaction").
///
/// ```bsx
/// <CreateActor name="Coach" kind="Agent" {(ModelStreamer{provider:OpenAi, size:Large}, Compact{template:Document("templates/Compactor.bsx")})}/>
/// <!-- templates/Compactor.bsx -->
/// <RepeatWhileFunctionCallOutput>
/// 	<Thread {Sequence}>
/// 		<CompactInstructions/>
/// 		<CreateActor name="Reader" kind="User"/>
/// 		<CreateActor name="Compactor" kind="Agent" {ModelStreamer{provider:Gemini, size:Small}}>
/// 			<Router><ReadPosts/></Router>
/// 		</CreateActor>
/// 	</Thread>
/// </RepeatWhileFunctionCallOutput>
/// ```
///
/// Before each of the agent's model calls ([`Self::prepare`]), inside its
/// turn so an interrupt drops a compaction and leaves the older one in force:
///
/// - **Refresh.** Its newest compaction, if stale, is refreshed: one call
///   told the summary and what changed under it since, however old the
///   posts and however many changed.
/// - **Compact.** Past `at` tokens ([`ThreadRecords::context_tokens`]), the
///   posts older than the newest `keep` tokens are folded in, no more than
///   `step` tokens a call, each call a stored compaction so a crash resumes.
///   The boundary never falls between a call and its output or on a post in
///   progress. Past `at` while stale, the first call does both.
///
/// Each answer is a `compaction` post ([`Compaction`]) authored by the
/// agent, its subject the last post it covers, its [`ResponseMeta`] the
/// compacting model's, its usage the sum of that thread's responses and its
/// `read` the newest post of the conversation its message was built at. A
/// failure is the turn's error post, the turn ending and never answering
/// over a stale summary, and the next turn tries again.
///
/// A template holding a [`UserInput`] is refused. One that mounts a store
/// keeps the originals in that store, as ordinary posts any view of it
/// shows, which nothing shipped does. Give a compactor no tool that reaches
/// outside: it is the one agent holding the originals.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = Compact::on_add)]
pub struct Compact {
	/// The template building the compaction thread, required so the
	/// compacting model is always named.
	pub template: TemplateSource,
	/// The input tokens past which the context compacts.
	pub at: u64,
	/// The newest tokens left verbatim.
	pub keep: u64,
	/// The most tokens of posts one compaction is given, by default `at`:
	/// set lower for a compacting model with a smaller context.
	pub step: Option<u64>,
	/// Whether each compaction folds into the last or reads everything.
	pub method: CompactMethod,
	/// Whether the seed is held apart or folded into the summary.
	pub seed: CompactSeed,
	/// How many tokens back a first compaction reads, everything when
	/// unset; the summary then says where it starts.
	pub backfill: Option<u64>,
}

/// How a [`Compact`] folds posts in.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Default)]
pub enum CompactMethod {
	/// Each compaction is given the newest summary, what changed under it,
	/// and the posts after it.
	#[default]
	Rolling,
	/// Each compaction is given every post back to the first compaction's
	/// start, no summary and no changes, in one call, failing as the
	/// provider does once that no longer fits. Its window holds the whole
	/// thread.
	Full,
}

/// Where a [`Compact`] keeps the seed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Default)]
pub enum CompactSeed {
	/// Held apart, read verbatim before the summary.
	#[default]
	Keep,
	/// Folded into the first compaction.
	Compact,
}

/// On the build root of a compaction thread: the window whose agent it
/// compacts, where a [`ReadPosts`] in it reads everything.
#[derive(Debug, Clone, Copy, Component)]
pub struct Compacting(pub Entity);

impl Default for Compact {
	fn default() -> Self {
		Self {
			template: TemplateSource::template(""),
			at: 100_000,
			keep: 20_000,
			step: None,
			method: default(),
			seed: default(),
			backfill: None,
		}
	}
}

impl Compact {
	/// Compact through `template`.
	pub fn new(template: TemplateSource) -> Self {
		Self {
			template,
			..default()
		}
	}

	/// Refuse a [`Compact`] naming no template as it lands.
	fn on_add(
		mut world: bevy::ecs::world::DeferredWorld,
		cx: bevy::ecs::lifecycle::HookContext,
	) {
		if let Some(Err(err)) = world
			.get::<Compact>(cx.entity)
			.map(|compact| compact.assert_template())
		{
			world.commands().handle_command_error::<Compact>(err);
		}
	}

	/// Refuse a template naming nothing.
	fn assert_template(&self) -> Result {
		match &self.template {
			TemplateSource::Template(name) if name.as_str().is_empty() => {
				bevybail!(
					"`Compact` names no template, so no model would compact: name the thread that does, ie `Compact{{template:Document(\"templates/Compactor.bsx\")}}`"
				)
			}
			_ => Ok(()),
		}
	}

	/// The most tokens one compaction is given.
	fn step(&self) -> u64 { self.step.unwrap_or(self.at).max(1) }

	/// Refresh or compact the context of the agent at `caller` before its
	/// next model call, as its [`Compact`] asks, if it carries one.
	///
	/// ## Errors
	///
	/// Errors when a compaction thread fails or answers nothing, so the
	/// turn ends on the error and never reads a stale summary.
	pub async fn prepare(caller: &AsyncEntity) -> Result {
		let Ok(compact) = caller.get_cloned::<Compact>().await else {
			return Ok(());
		};
		compact.assert_template()?;
		let planned = compact.clone();
		let Some((window, plan)) = caller
			.with_state::<ThreadQuery, _>(move |entity, query| -> Result<_> {
				let (window, _, records) = query.thread_and_records(entity)?;
				let reader = query.actor_id(entity)?;
				let cites = query.tools(entity)?.iter().any(|(_, tool)| {
					matches!(tool, ToolDefinition::Function(func)
							if func.path() == ReadPosts::PATH)
				});
				CompactPlan::new(&planned, records, &reader, cites)
					.map(|plan| (window, plan))
					.xok()
			})
			.await??
		else {
			return Ok(());
		};
		let plan = plan.read_missing(&caller.world().entity(window)).await?;
		plan.run(caller, window, &compact).await
	}
}

/// What one round of compaction does, read off the records before any call.
struct CompactPlan {
	/// The newest compaction, stale or not, and its body.
	previous: Option<(PostId, Compaction)>,
	/// The posts changed under the newest compaction since: each post, the
	/// first log changing it since, and the post as it stands, `None` until
	/// read from the store.
	changes: Vec<(PostId, Post, Option<Post>)>,
	/// Each run of posts given to one compaction, in order, empty for a
	/// refresh.
	chunks: Vec<Vec<Arc<Post>>>,
	/// The method, whether the seed is inside, and whether the conversation's
	/// agent can read a post by id.
	method: CompactMethod,
	seed: bool,
	cites: bool,
	/// The first post read when `backfill` cut the first compaction short.
	backfilled: Option<PostId>,
	/// The newest post of the conversation the message is built at.
	read: Option<PostId>,
	/// Each actor by id, to name a post's author.
	actors: Vec<Actor>,
}

impl CompactPlan {
	/// The plan of `reader`'s context in `records` as `compact` asks: a
	/// refresh when its newest compaction is stale, a compaction past `at`,
	/// else nothing.
	fn new(
		compact: &Compact,
		records: &ThreadRecords,
		reader: &ActorId,
		cites: bool,
	) -> Option<Self> {
		let shown = records.shown(..);
		let newest = records.newest_compaction(reader);
		let previous = newest
			.and_then(|post| (post.id(), post.log::<Compaction>()?).xsome());
		let changes = newest
			.map(|post| records.changes_under(post))
			.unwrap_or_default()
			.into_iter()
			.filter_map(|(changed, log)| {
				let log = records.post(log)?.clone();
				(changed, log, records.post(changed).cloned()).xsome()
			})
			.collect::<Vec<_>>();
		let stale = !changes.is_empty();
		let subject = newest.and_then(Post::subject);
		let seed = previous
			.as_ref()
			.map_or(compact.seed == CompactSeed::Compact, |(_, body)| {
				body.seed
			});
		let seed_floor = (!seed).then(|| records.seed()).flatten();
		// the tokens of the context, a stale summary standing as it was
		let tokens = records.context_tokens_through(
			&shown,
			reader,
			newest.map(|post| (seed_floor, post)),
		);
		let readable = |post: &&Arc<Post>| {
			post.kind() != &PostKind::Compaction
				&& seed_floor.is_none_or(|seed| post.id() > seed)
		};
		let mut plan = Self {
			previous,
			changes,
			chunks: Vec::new(),
			method: compact.method,
			seed,
			cites,
			backfilled: None,
			read: records.read_through(),
			actors: records.actors().to_vec(),
		};
		if tokens <= compact.at {
			return stale.then_some(plan).map(|plan| plan.refreshing(records));
		}
		// the posts read verbatim, oldest first
		let verbatim = shown
			.iter()
			.filter(readable)
			.filter(|post| subject.is_none_or(|subject| post.id() > subject))
			.cloned()
			.collect::<Vec<_>>();
		// keep the newest `keep` tokens, fold the rest
		let mut kept = 0;
		let mut end = verbatim.len();
		while end > 0 {
			kept += ThreadRecords::estimate(&verbatim[end - 1]);
			if kept > compact.keep {
				break;
			}
			end -= 1;
		}
		let Some(end) = Self::boundary(&verbatim, end.checked_sub(1)) else {
			return stale.then_some(plan).map(|plan| plan.refreshing(records));
		};
		let mut fold = verbatim[..=end].to_vec();
		// a first compaction reads only `backfill` tokens back
		if plan.previous.is_none()
			&& let Some(backfill) = compact.backfill
		{
			let mut read = 0;
			let start = fold
				.iter()
				.rposition(|post| {
					read += ThreadRecords::estimate(post);
					read > backfill
				})
				.map_or(0, |index| index + 1)
				.min(end);
			if start > 0 {
				plan.backfilled = Some(fold[start].id());
				fold.drain(..start);
			}
		}
		plan.chunks = match compact.method {
			CompactMethod::Rolling => Self::chunks(fold, compact.step()),
			// every post back to the first compaction's start
			CompactMethod::Full => {
				let from = plan
					.previous
					.as_ref()
					.and_then(|(_, body)| body.from)
					.or_else(|| fold.first().map(|post| post.id()));
				let through = fold.last().map(|post| post.id());
				vec![
					shown
						.iter()
						.filter(readable)
						.filter(|post| {
							from.is_none_or(|from| post.id() >= from)
								&& through
									.is_some_and(|through| post.id() <= through)
						})
						.cloned()
						.collect(),
				]
			}
		};
		plan.xsome()
	}

	/// This plan as a refresh: given no posts, except by [`CompactMethod::Full`],
	/// which reads its whole span again.
	fn refreshing(mut self, records: &ThreadRecords) -> Self {
		if self.method == CompactMethod::Full
			&& let Some((id, body)) = &self.previous
		{
			let subject = records.post(*id).and_then(Post::subject);
			let shown = records.shown(..);
			let seed_floor = (!self.seed).then(|| records.seed()).flatten();
			self.chunks = vec![
				shown
					.iter()
					.filter(|post| post.kind() != &PostKind::Compaction)
					.filter(|post| {
						seed_floor.is_none_or(|seed| post.id() > seed)
					})
					.filter(|post| {
						body.from.is_none_or(|from| post.id() >= from)
							&& subject
								.is_some_and(|subject| post.id() <= subject)
					})
					.cloned()
					.collect(),
			];
		}
		self
	}

	/// The last index of `posts` at or before `end` a compaction may stop
	/// at: never on a post in progress, nor between a call and its output,
	/// a call not yet answered being open.
	fn boundary(posts: &[Arc<Post>], end: Option<usize>) -> Option<usize> {
		let mut end = end?;
		loop {
			let held = &posts[..=end];
			let cut = held.iter().enumerate().find_map(|(at, post)| {
				let open_call = post.kind() == &PostKind::ToolCall
					&& !held.iter().any(|output| {
						output.kind() == &PostKind::ToolOutput
							&& output.subject() == Some(post.id())
					});
				(post.in_progress() || open_call).then_some(at)
			});
			match cut {
				None => return Some(end),
				Some(0) => return None,
				Some(at) => end = at - 1,
			}
		}
	}

	/// `fold` in runs of at most `step` tokens, at least one post each,
	/// each ending where a compaction may stop.
	fn chunks(fold: Vec<Arc<Post>>, step: u64) -> Vec<Vec<Arc<Post>>> {
		let mut chunks = Vec::new();
		let mut rest = fold;
		while !rest.is_empty() {
			let mut tokens = 0;
			let fits = rest
				.iter()
				.position(|post| {
					tokens += ThreadRecords::estimate(post);
					tokens > step
				})
				.unwrap_or(rest.len());
			let end = Self::boundary(&rest, fits.max(1).checked_sub(1))
				.or_else(|| Self::boundary(&rest, Some(rest.len() - 1)))
				.unwrap_or(rest.len() - 1);
			let tail = rest.split_off(end + 1);
			chunks.push(rest);
			rest = tail;
		}
		chunks
	}

	/// Read each changed post the records lack from the window's store, a
	/// post older than its run.
	async fn read_missing(mut self, window: &AsyncEntity) -> Result<Self> {
		if self.changes.iter().all(|(_, _, post)| post.is_some()) {
			return self.xok();
		}
		let (store, thread) = window
			.with_state::<Query<(Option<&ThreadStore>, &Thread)>, _>(
				|entity, query| -> Result<_> {
					let (store, thread) = query.get(entity)?;
					(store.cloned(), thread.clone()).xok()
				},
			)
			.await??;
		for (changed, _, post) in &mut self.changes {
			if post.is_none()
				&& let Some(store) = &store
			{
				*post = store.read_post(&thread, *changed).await?;
			}
		}
		self.xok()
	}

	/// Run each compaction of the plan in order, each written as it lands.
	async fn run(
		mut self,
		caller: &AsyncEntity,
		window: Entity,
		compact: &Compact,
	) -> Result {
		let chunks = core::mem::take(&mut self.chunks);
		if chunks.is_empty() {
			// a refresh, standing over what the stale one covered
			let Some((id, body)) = self.previous.clone() else {
				return Ok(());
			};
			let subject = caller
				.with_state::<ThreadQuery, _>(move |entity, query| {
					query
						.records(entity)
						.ok()
						.and_then(|records| records.post(id))
						.and_then(Post::subject)
				})
				.await?;
			let Some(subject) = subject else {
				return Ok(());
			};
			let message = self.message(&[]);
			let answer = Self::call(caller, window, compact, message).await?;
			return self
				.write(caller, answer, subject, Compaction {
					text: String::new(),
					from: body.from,
					previous: Some(id),
					seed: body.seed,
				})
				.await;
		}
		for chunk in chunks {
			let (Some(from), Some(subject)) = (
				chunk.first().map(|post| post.id()),
				chunk.last().map(|post| post.id()),
			) else {
				continue;
			};
			let message = self.message(&chunk);
			let answer = Self::call(caller, window, compact, message).await?;
			let previous = match self.method {
				CompactMethod::Rolling => {
					self.previous.as_ref().map(|(id, _)| *id)
				}
				CompactMethod::Full => None,
			};
			let id = self
				.write_answer(caller, answer, subject, Compaction {
					text: String::new(),
					from: Some(from),
					previous,
					seed: self.seed,
				})
				.await?;
			// the next run folds into this one, changes told once
			self.changes.clear();
			self.backfilled = None;
			self.previous = Some(id);
		}
		Ok(())
	}

	/// The message a compaction thread is entered with, each part only when
	/// it holds something: the newest summary, what changed under it since,
	/// and `posts` to fold in, each with its id.
	fn message(&self, posts: &[Arc<Post>]) -> String {
		let mut parts = Vec::new();
		if self.method == CompactMethod::Rolling
			&& let Some((_, body)) = &self.previous
		{
			parts.push(format!("<summary>\n{}\n</summary>", body.text));
		}
		if self.method == CompactMethod::Rolling && !self.changes.is_empty() {
			let changes = self
				.changes
				.iter()
				.map(|(changed, log, post)| {
					// what the summary read: an edit's original, else the
					// tombstone's body, which a deletion keeps
					let was = post
						.as_ref()
						.map(|post| {
							let mut was = post.clone();
							was.set_status(PostStatus::Completed);
							if let Some(edited) = log.log::<PostEdited>() {
								was.set_body(edited.was);
							}
							Compaction::post_text(&was)
						})
						.unwrap_or_else(|| RequestBlobs::UNAVAILABLE.to_string());
					let now = match post {
						Some(post) if !post.is_deleted() => Compaction::post_text(post),
						_ => "(deleted)".to_string(),
					};
					format!(
						"<change post=\"{changed}\">\n<was>{was}</was>\n<now>{now}</now>\n</change>"
					)
				})
				.collect::<Vec<_>>()
				.join("\n");
			parts.push(format!("<changes>\n{changes}\n</changes>"));
		}
		if let Some(first) = self.backfilled {
			parts.push(format!(
				"Nothing before post {first} is given: say in the summary that it starts there."
			));
		}
		if !posts.is_empty() {
			let posts = posts
				.iter()
				.filter_map(|post| self.post_line(post))
				.collect::<Vec<_>>()
				.join("\n");
			parts.push(format!("<posts>\n{posts}\n</posts>"));
		}
		if self.cites {
			parts.push(
				"The conversation's agent reads a post by its id, so cite the id of each post worth reading again."
					.to_string(),
			);
		}
		parts.join("\n\n")
	}

	/// One post of the message, as shown ([`Compaction::post_line`]).
	fn post_line(&self, post: &Post) -> Option<String> {
		let author = self
			.actors
			.iter()
			.find(|actor| actor.id() == post.author())
			.map_or(post.author().as_str(), |actor| actor.name());
		Compaction::post_line(post, author, false)
	}

	/// Run one compaction thread for `message` beside the window `window`,
	/// answering its last text and the metadata of its responses.
	async fn call(
		caller: &AsyncEntity,
		window: Entity,
		compact: &Compact,
		message: String,
	) -> Result<CompactAnswer> {
		let world = caller.world();
		let source = compact.template.clone().load(caller).await?;
		let parent = world
			.with(move |world: &mut World| {
				world.get::<ChildOf>(window).map(ChildOf::parent)
			})
			.await;
		let parts = Request::get("compact").parts().clone();
		let function = ThreadFunction::build(
			&world,
			window,
			parent,
			source,
			parts.clone(),
		)
		.await?;
		world
			.entity(function.root())
			.insert(Compacting(window))
			.await?;
		let entry = ThreadEntry::new(parts, ThreadEntryParams {
			new: true,
			..default()
		})
		.with_text(message, None);
		let answer = match function.run(entry).await {
			Ok(ThreadOutcome {
				outcome: Outcome::Pass(_),
				thread,
			}) => {
				world
					.with(move |world: &mut World| {
						CompactAnswer::of(world, thread)
					})
					.await
			}
			Ok(ThreadOutcome { .. }) => Err(bevyhow!(
				"the compaction thread `{}` failed",
				function.name()
			)),
			Err(err) => Err(err),
		};
		function.despawn().await;
		answer
	}

	/// Write `answer` as a compaction by the agent at `caller`, `body` its
	/// fields but the text, which is the answer's.
	async fn write(
		&self,
		caller: &AsyncEntity,
		answer: CompactAnswer,
		subject: PostId,
		body: Compaction,
	) -> Result {
		self.write_answer(caller, answer, subject, body)
			.await
			.map(|_| ())
	}

	/// As [`Self::write`], answering the compaction written and its body.
	async fn write_answer(
		&self,
		caller: &AsyncEntity,
		answer: CompactAnswer,
		subject: PostId,
		mut body: Compaction,
	) -> Result<(PostId, Compaction)> {
		body.text = match self.backfilled {
			Some(first) => format!(
				"(This summary starts at post {first}, {}: nothing before it was read.)\n{}",
				first.created().format_iso8601_mins(),
				answer.text
			),
			None => answer.text,
		};
		let read = self.read;
		let written = body.clone();
		caller
			.with_state::<ThreadRecordsQuery, _>(
				move |entity, mut query| -> Result<PostId> {
					let actor = query.actor_id(entity)?;
					let thread = query.thread_id(entity)?;
					let post = Post::new_log(actor, thread, &body)?
						.with_subject(subject);
					let id = post.id();
					let mut records = query.records_mut(entity)?;
					records.set_meta(ResponseMeta {
						post_id: id,
						read,
						..answer.meta
					});
					records.upsert_post(post);
					id.xok()
				},
			)
			.await?
			.map(|id| (id, written))
	}
}

/// What a compaction thread answered: its last text, and its responses'
/// metadata, the newest's model and the sum of their usage.
struct CompactAnswer {
	text: String,
	meta: ResponseMeta,
}

impl CompactAnswer {
	/// The answer of the compaction thread `thread`.
	fn of(world: &mut World, thread: Entity) -> Result<Self> {
		let text =
			ThreadFunction::last_text(world, thread).ok_or_else(|| {
				bevyhow!("the compaction thread answered nothing")
			})?;
		let records = world.get::<ThreadRecords>(thread).ok_or_else(|| {
			bevyhow!("the compaction thread holds no records")
		})?;
		let newest = records
			.posts()
			.iter()
			.rev()
			.find_map(|post| records.meta(post.id()))
			.cloned()
			.unwrap_or_default();
		let usage =
			records.metas().filter_map(|meta| meta.usage).sum::<Usage>();
		Self {
			text,
			meta: ResponseMeta {
				usage: Some(usage),
				// a compaction starts a new chain
				response_stored: false,
				..newest
			},
		}
		.xok()
	}
}

#[cfg(all(test, feature = "agent"))]
pub(crate) mod test {
	use super::CompactPlan;
	use crate::prelude::*;
	use crate::streaming::request_test;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_scene::prelude::*;

	/// A compactor answering its message back as the summary, so a summary
	/// says what it was given.
	pub(crate) const ECHO: &str = r#"
<Thread {Sequence}>
	<CompactInstructions/>
	<CreateActor name="Reader" kind="User"/>
	<CreateActor name="Compactor" kind="Agent" {MockPostStreamer}/>
</Thread>
"#;

	/// A compactor answering one fixed summary, as a model told to leave out
	/// what was taken back.
	pub(crate) const FIXED: &str = r#"
<Thread {Sequence}>
	<CompactInstructions/>
	<CreateActor name="Reader" kind="User"/>
	<CreateActor name="Compactor" kind="Agent" {MockPostStreamer{custom_response:"FIXED-SUMMARY"}}/>
</Thread>
"#;

	/// A compactor whose model fails.
	pub(crate) const FAILING: &str = r#"
<Thread {Sequence}>
	<CreateActor name="Reader" kind="User"/>
	<CreateActor name="Compactor" kind="Agent" {Bail}/>
</Thread>
"#;

	/// A compactor held until its gate is released.
	pub(crate) const HELD: &str = r#"
<Thread {Sequence}>
	<CreateActor name="Reader" kind="User"/>
	<CreateActor name="Compactor" kind="Agent" {MockPostStreamer{script:[Hold("HELD-SUMMARY")]}}/>
</Thread>
"#;

	/// A compactor that does as it is told: its summary is its message less
	/// every original it was told of, with what now stands in its place.
	pub(crate) const SCRUB: &str = r#"
<Thread {Sequence}>
	<CreateActor name="Reader" kind="User"/>
	<CreateActor name="Compactor" kind="Agent" {Scrub}/>
</Thread>
"#;

	/// Answer the last message in the thread as [`scrubbed`] would.
	async fn scrub(cx: ActionContext) -> Result<Outcome> {
		cx.caller
			.with_state::<ThreadRecordsQuery, _>(
				|entity, mut query| -> Result {
					let actor = query.actor_id(entity)?;
					let thread = query.thread_id(entity)?;
					let records = query.records(entity)?;
					let message = records
						.content_posts()
						.filter(|view| view.actor.kind() == ActorKind::User)
						.last()
						.map(|view| view.post.to_string())
						.unwrap_or_default();
					query.push_post(
						entity,
						AgentPost::new_text(
							actor,
							thread,
							scrubbed(&message),
							PostStatus::Completed,
						),
					)
				},
			)
			.await??;
		Outcome::PASS.xok()
	}
	#[derive(Component, Reflect, Default)]
	#[reflect(Component, Default)]
	#[require(Action<(), Outcome> = Action::new_async(scrub))]
	struct Scrub;

	/// `message` less its `<changes>`, each original they name taken out
	/// wherever it is, what now stands appended.
	fn scrubbed(message: &str) -> String {
		let between = |text: &str, open: &str, close: &str| -> Vec<String> {
			text.split(open)
				.skip(1)
				.filter_map(|rest| {
					rest.split_once(close).map(|(inner, _)| inner.to_string())
				})
				.collect()
		};
		let mut summary = match message.split_once("<changes>") {
			Some((before, rest)) => format!(
				"{before}{}",
				rest.split_once("</changes>").map_or("", |(_, after)| after)
			),
			None => message.to_string(),
		};
		for was in between(message, "<was>", "</was>") {
			summary = summary.replace(&was, "");
		}
		for now in between(message, "<now>", "</now>")
			.into_iter()
			.filter(|now| now != "(deleted)")
		{
			summary.push_str(&format!("\n{now}"));
		}
		summary
	}

	/// A behavior that errors.
	async fn bail(_cx: ActionContext) -> Result<Outcome> { bevybail!("boom") }
	#[derive(Component, Reflect, Default)]
	#[reflect(Component, Default)]
	#[require(Action<(), Outcome> = Action::new_async(bail))]
	struct Bail;

	pub(crate) const ANN: ActorId = actor_id!("ann");
	pub(crate) const COACH: ActorId = actor_id!("coach");

	/// A conversation whose root holds the store its compactors are documents of.
	pub(crate) struct Chat {
		pub app: App,
		pub root: Entity,
		pub window: Entity,
	}

	impl Chat {
		/// A conversation of a System seed, Ann and the agent Coach over
		/// a store, Coach carrying `compact`, each compactor above a
		/// document of the store, entered.
		pub async fn new(compact: &str) -> Self {
			Self::of(&Self::scene(compact)).await
		}

		/// As [`Self::new`], each error raised outside a result a warning,
		/// ie a refusal as a component lands.
		pub async fn warned(compact: &str) -> Self {
			Self::build(&Self::scene(compact), true).await
		}

		/// The conversation [`Self::new`] spawns.
		fn scene(compact: &str) -> String {
			format!(
				r#"<Thread {{(Sequence, MountThreadStore{{path:"chat"}})}}>
	<CreateActor name="System" kind="System"><CreatePost text="SEED-RULE"/></CreateActor>
	<CreateActor name="Ann" kind="User"/>
	<CreateActor name="Coach" kind="Agent" {{(MockPostStreamer{{custom_response:"ok"}}, {compact})}}/>
</Thread>"#
			)
		}

		/// The conversation `source` over a store holding each compactor,
		/// entered.
		pub async fn of(source: &str) -> Self {
			Self::build(source, false).await
		}

		/// As [`Self::of`], each error raised outside a result a warning when
		/// `warned`.
		async fn build(source: &str, warned: bool) -> Self {
			let mut app = App::new();
			app.add_plugins(MinimalPlugins)
				.install_types::<ThreadPlugin>()
				.register_type::<Bail>()
				.register_type::<Scrub>();
			if warned {
				app.world_mut().insert_resource(
					bevy::ecs::error::FallbackErrorHandler(
						bevy::ecs::error::warn,
					),
				);
			}
			let store = BlobStore::temp();
			for (name, source) in [
				("Echo", ECHO),
				("Fixed", FIXED),
				("Failing", FAILING),
				("Held", HELD),
				("Scrub", SCRUB),
			] {
				store
					.insert(&FwdPath::new(format!("{name}.bsx")).unwrap(), source)
					.await
					.unwrap();
			}
			let root = app
				.world_mut()
				.spawn((
					store.clone(),
					JsonTableStore::over(store.clone()).unwrap(),
				))
				.id();
			let window = BsxParser::spawn(app.world_mut(), source).unwrap();
			app.world_mut().entity_mut(window).insert(ChildOf(root));
			request_test::enter(&mut app, window).await;
			Self { app, root, window }
		}

		/// The behavior entity of `actor`.
		pub fn actor(&mut self, actor: &ActorId) -> Entity {
			request_test::actor_entity(&mut self.app, actor)
		}

		/// The window's records.
		pub fn records(&self) -> &ThreadRecords {
			self.app.world().get::<ThreadRecords>(self.window).unwrap()
		}

		/// `author` says `text`, answering the post.
		pub fn say(&mut self, author: &ActorId, text: &str) -> Post {
			let mut window = self.app.world_mut().entity_mut(self.window);
			let thread = window.get::<Thread>().unwrap().id();
			let post = AgentPost::new_text(
				author.clone(),
				thread,
				text,
				PostStatus::Completed,
			);
			window
				.get_mut::<ThreadRecords>()
				.unwrap()
				.upsert_post(post.clone());
			post
		}

		/// Ann says each of `texts`.
		pub fn said(
			&mut self,
			texts: impl IntoIterator<Item = impl AsRef<str>>,
		) {
			for text in texts {
				self.say(&ANN, text.as_ref());
			}
		}

		/// Run one turn of `actor`.
		pub async fn turn(&mut self, actor: &ActorId) {
			let entity = self.actor(actor);
			self.app
				.world_mut()
				.entity_mut(entity)
				.call::<(), Outcome>(())
				.await
				.unwrap();
		}

		/// The compactions `author` wrote, oldest first.
		pub fn compactions(&self, author: &ActorId) -> Vec<(Post, Compaction)> {
			self.records()
				.posts()
				.iter()
				.filter(|post| {
					post.kind() == &PostKind::Compaction
						&& post.author() == author
				})
				.map(|post| {
					(Post::clone(post), post.log::<Compaction>().unwrap())
				})
				.collect()
		}

		/// Both protocols' requests of `actor`'s model, as json text.
		pub async fn requests(&mut self, actor: &ActorId) -> [String; 2] {
			let entity = self.actor(actor);
			self.requests_of(entity).await
		}

		/// `actor` edits `post` to `text`.
		pub async fn edit(
			&mut self,
			actor: &ActorId,
			post: PostId,
			text: &str,
		) {
			let (actor, text, window) =
				(actor.clone(), text.to_string(), self.window);
			self.app
				.world_mut()
				.run_async_local_then(async move |world: AsyncWorld| {
					EditPost::new(actor, post, text)
						.apply(&world.entity(window))
						.await
				})
				.await
				.unwrap();
		}

		/// `actor` deletes `post`.
		pub async fn delete(&mut self, actor: &ActorId, post: PostId) {
			let (actor, window) = (actor.clone(), self.window);
			self.app
				.world_mut()
				.run_async_local_then(async move |world: AsyncWorld| {
					DeletePost::new(actor, post)
						.apply(&world.entity(window))
						.await
				})
				.await
				.unwrap();
		}

		/// Run a turn of `actor` in a task, answering whether it ended.
		pub fn spawn_turn(&mut self, actor: &ActorId) -> Store<bool> {
			let entity = self.actor(actor);
			let done = Store::default();
			self.app.world_mut().run_async(
				async move |world: AsyncWorld| -> Result {
					world.entity(entity).call::<(), Outcome>(()).await?;
					done.set(true);
					Ok(())
				},
			);
			done
		}

		/// Update until `holds`, naming `what` if it never does.
		pub fn pump_until(
			&mut self,
			what: &str,
			mut holds: impl FnMut(&mut World) -> bool,
		) {
			for _ in 0..500 {
				if holds(self.app.world_mut()) {
					return;
				}
				self.app.update();
			}
			panic!("never {what}");
		}

		/// Update until a held compactor is streaming, answering its gate.
		pub fn pump_until_held(&mut self) -> MockGate {
			let coach = self.actor(&COACH);
			let mut gate = None;
			self.pump_until("held a compaction", |world| {
				gate = world
					.query::<(Entity, &MockPostStreamer)>()
					.iter(world)
					.find(|(entity, mock)| {
						*entity != coach && !mock.script.is_empty()
					})
					.map(|(_, mock)| mock.gate().clone());
				gate.is_some()
					&& world
						.query::<&ThreadRecords>()
						.iter(world)
						.any(|records| records.in_progress().next().is_some())
			});
			gate.unwrap()
		}

		/// Write a compaction by `author` covering everything through
		/// `subject`, its message built at the newest post.
		pub fn compacted(
			&mut self,
			author: &ActorId,
			subject: PostId,
			text: &str,
		) -> Post {
			let mut window = self.app.world_mut().entity_mut(self.window);
			let thread = window.get::<Thread>().unwrap().id();
			let mut records = window.get_mut::<ThreadRecords>().unwrap();
			let post = Post::new_log(author.clone(), thread, &Compaction {
				text: text.into(),
				from: records.posts().first().map(|post| post.id()),
				..default()
			})
			.unwrap()
			.with_subject(subject);
			let read = records.read_through();
			records.set_meta(ResponseMeta {
				post_id: post.id(),
				read,
				..default()
			});
			records.upsert_post(post.clone());
			post
		}

		/// The behavior entity of `actor` in the window `window`.
		pub fn actor_in(&mut self, window: Entity, actor: &ActorId) -> Entity {
			let world = self.app.world_mut();
			world
				.query::<(Entity, &ActorRef)>()
				.iter(world)
				.filter(|(_, actor_ref)| &actor_ref.0 == actor)
				.map(|(entity, _)| entity)
				.collect::<Vec<_>>()
				.into_iter()
				.find(|entity| {
					world.with_state::<Query<&ChildOf>, _>(|ancestors| {
						ancestors
							.iter_ancestors(*entity)
							.any(|ancestor| ancestor == window)
					})
				})
				.unwrap()
		}

		/// Spawn the conversation `source` again beside this one, entered
		/// with `request`, answering its window.
		pub async fn enter_again(
			&mut self,
			source: &str,
			request: Request,
		) -> Entity {
			let window =
				BsxParser::spawn(self.app.world_mut(), source).unwrap();
			self.app
				.world_mut()
				.entity_mut(window)
				.insert(ChildOf(self.root));
			self.app
				.world_mut()
				.run_async_local_then(
					async move |world: AsyncWorld| -> Result {
						Thread::enter(
							world.clone(),
							window,
							ThreadEntry::parse(request.parts().clone())?,
						)
						.await
					},
				)
				.await
				.unwrap();
			window
		}

		/// Wait for the window's records to reach its store.
		pub async fn flush(&mut self) {
			let window = self.window;
			self.app
				.world_mut()
				.run_async_local_then(async move |world: AsyncWorld| {
					ThreadStore::flush(&world.entity(window)).await
				})
				.await
				.unwrap();
		}

		/// Both protocols' requests of the behavior entity `agent`, as json
		/// text.
		pub async fn requests_of(&mut self, agent: Entity) -> [String; 2] {
			request_test::requests(&mut self.app, agent)
				.await
				.map(|request| request.to_json_string())
		}

		/// Whether no thread but the conversation is spawned.
		pub fn alone(&mut self) -> bool {
			self.app
				.world_mut()
				.query_filtered::<(), With<Thread>>()
				.iter(self.app.world())
				.count() == 1
		}
	}

	/// Words of `count` posts, each a distinct marker, ie `POST-3`.
	pub(crate) fn posts(range: core::ops::Range<usize>) -> Vec<String> {
		range
			.map(|index| format!("POST-{index} {}", "word ".repeat(10)))
			.collect()
	}

	/// Past `at`, the posts older than the newest `keep` tokens fold into one
	/// compaction by the agent, its context then the seed, the summary's note
	/// and the posts after it, and its meta the compactor's.
	#[beet_core::test]
	async fn compacts_past_at() {
		let mut chat = Chat::new(
			r#"Compact{template:Document("Echo.bsx"), at:200, keep:60}"#,
		)
		.await;
		chat.said(posts(0..12));
		chat.turn(&COACH).await;
		let compactions = chat.compactions(&COACH);
		compactions.len().xpect_eq(1);
		let (post, body) = &compactions[0];
		body.previous.xpect_none();
		body.seed.xpect_false();
		body.text
			.as_str()
			.xpect_contains("POST-0")
			.xnot()
			.xpect_contains("SEED-RULE");
		let meta = chat.records().meta(post.id()).unwrap().clone();
		meta.response_stored.xpect_false();
		meta.usage.unwrap().input_tokens.xpect_greater_than(0);
		meta.read.xpect_some();
		chat.alone().xpect_true();
		// the context: the seed, the summary, then what follows it verbatim
		let subject = post.subject().unwrap();
		let folded = chat
			.records()
			.posts()
			.iter()
			.filter(|post| post.id() <= subject && post.author() == &ANN)
			.count();
		folded.xpect_greater_than(0);
		for request in chat.requests(&COACH).await {
			request
				.as_str()
				.xpect_contains("SEED-RULE")
				.xpect_contains("A summary of the conversation");
			request.as_str().xpect_contains("POST-11");
		}
	}

	/// The readable posts of `records`: neither a log nor the seed.
	fn readable(records: &ThreadRecords) -> Vec<PostId> {
		records
			.posts()
			.iter()
			.filter(|post| !post.is_log())
			.filter(|post| records.seed().is_none_or(|seed| post.id() > seed))
			.map(|post| post.id())
			.collect()
	}

	/// Each compaction of `author` picks up where the one it folds in left
	/// off, so no readable post falls between two: the first from the first
	/// post unless backfill cut it, each next from the first post after the
	/// last one's subject.
	pub(crate) fn assert_no_gap(records: &ThreadRecords, author: &ActorId) {
		let readable = readable(records);
		for post in records
			.posts()
			.iter()
			.filter(|post| post.kind() == &PostKind::Compaction)
			.filter(|post| post.author() == author)
		{
			let body = post.log::<Compaction>().unwrap();
			let Some(from) = body.from else {
				// a refresh covers what the one it folds in did
				continue;
			};
			let floor = body
				.previous
				.and_then(|previous| records.post(previous))
				.and_then(Post::subject);
			let gap = readable
				.iter()
				.filter(|id| {
					floor.is_none_or(|floor| **id > floor) && **id < from
				})
				.count();
			match body.previous {
				Some(_) => {
					gap.xpect_eq(0);
				}
				None if gap > 0 => {
					body.text
						.as_str()
						.xpect_starts_with("(This summary starts at post");
				}
				None => {}
			}
		}
	}

	/// Past `step`, a span folds in several compactions in one turn, each
	/// folding in the last.
	#[beet_core::test]
	async fn a_span_past_step_is_taken_in_steps() {
		let mut chat = Chat::new(
			r#"Compact{template:Document("Echo.bsx"), at:200, keep:60, step:50}"#,
		)
		.await;
		chat.said(posts(0..12));
		chat.turn(&COACH).await;
		let compactions = chat.compactions(&COACH);
		compactions.len().xpect_greater_than(1);
		for pair in compactions.windows(2) {
			pair[1].1.previous.xpect_eq(Some(pair[0].0.id()));
			pair[1].1.text.as_str().xpect_contains("<summary>");
		}
		assert_no_gap(chat.records(), &COACH);
		chat.alone().xpect_true();
	}

	/// `Full` folds in nothing: each compaction reads every post back to
	/// the first one's start, with no summary.
	#[beet_core::test]
	async fn full_reads_everything_again() {
		let mut chat = Chat::new(
			r#"Compact{template:Document("Echo.bsx"), at:200, keep:60, method:Full}"#,
		)
		.await;
		chat.said(posts(0..12));
		chat.turn(&COACH).await;
		chat.said(posts(12..24));
		chat.turn(&COACH).await;
		let compactions = chat.compactions(&COACH);
		compactions.len().xpect_eq(2);
		for (_, body) in &compactions {
			body.previous.xpect_none();
			body.text.as_str().xnot().xpect_contains("<summary>");
			body.text.as_str().xpect_contains("POST-0 ");
		}
		compactions[1].1.from.xpect_eq(compactions[0].1.from);
	}

	/// `seed:Compact` folds the seed into the first summary, which then
	/// holds it; by default it is held apart, never folded.
	#[beet_core::test]
	async fn the_seed_is_kept_or_compacted() {
		for (seed, inside) in [("Keep", false), ("Compact", true)] {
			let mut chat = Chat::new(&format!(
				r#"Compact{{template:Document("Echo.bsx"), at:200, keep:60, seed:{seed}}}"#
			))
			.await;
			chat.said(posts(0..12));
			chat.turn(&COACH).await;
			let (_, body) = chat.compactions(&COACH).remove(0);
			body.seed.xpect_eq(inside);
			body.text.contains("SEED-RULE").xpect_eq(inside);
			// read once either way, apart or in the summary
			for request in chat.requests(&COACH).await {
				request.matches("SEED-RULE").count().xpect_eq(1);
			}
		}
	}

	/// `backfill` cuts a first compaction short, and its text says where it
	/// starts.
	#[beet_core::test]
	async fn backfill_says_where_it_starts() {
		let mut chat = Chat::new(
			r#"Compact{template:Document("Echo.bsx"), at:200, keep:60, backfill:40}"#,
		)
		.await;
		chat.said(posts(0..12));
		chat.turn(&COACH).await;
		let (_, body) = chat.compactions(&COACH).remove(0);
		body.text
			.as_str()
			.xpect_starts_with("(This summary starts at post")
			.xnot()
			.xpect_contains("POST-0 ");
		let first = chat
			.records()
			.posts()
			.iter()
			.find(|post| post.author() == &ANN)
			.unwrap()
			.id();
		(body.from.unwrap() > first).xpect_true();
		assert_no_gap(chat.records(), &COACH);
	}

	/// A `Compact` naming no template, or one that does not load, ends the
	/// turn on an error naming it, and nothing is answered.
	#[beet_core::test]
	async fn refuses_a_template_naming_nothing() {
		Compact::default()
			.assert_template()
			.unwrap_err()
			.to_string()
			.xpect_contains("names no template");
		for (compact, error) in [
			("Compact{at:10}", "names no template"),
			(
				r#"Compact{template:Document("Missing.bsx"), at:10, keep:0}"#,
				"Missing.bsx",
			),
		] {
			let mut chat = Chat::warned(compact).await;
			chat.said(posts(0..4));
			chat.turn(&COACH).await;
			let last = chat.records().last_post().unwrap().clone();
			last.kind().clone().xpect_eq(PostKind::Error);
			last.to_string().as_str().xpect_contains(error);
			chat.compactions(&COACH).xpect_empty();
		}
	}

	/// The boundary never falls on a post in progress, nor between a call
	/// and its output, a call not yet answered being open.
	#[beet_core::test]
	fn the_boundary_keeps_each_call_with_its_output() {
		let thread = ThreadId::now();
		let text = |status: PostStatus| {
			std::sync::Arc::new(AgentPost::new_text(
				COACH, thread, "hi", status,
			))
		};
		let call = AgentPost::new_function_call(
			COACH,
			thread,
			"search",
			"c1",
			"{}",
			PostStatus::Completed,
		);
		let output = AgentPost::new_function_call_output(
			COACH,
			thread,
			call.id(),
			"c1",
			"found",
			PostStatus::Completed,
		);
		let posts = vec![
			text(PostStatus::Completed),
			std::sync::Arc::new(call),
			text(PostStatus::Completed),
			std::sync::Arc::new(output),
			text(PostStatus::InProgress),
			text(PostStatus::Completed),
		];
		let boundary = |end| CompactPlan::boundary(&posts, Some(end));
		boundary(0).xpect_eq(Some(0));
		// on the call, or between it and its output, falls before the call
		boundary(1).xpect_eq(Some(0));
		boundary(2).xpect_eq(Some(0));
		boundary(3).xpect_eq(Some(3));
		// never on or past a post in progress
		boundary(5).xpect_eq(Some(3));
		CompactPlan::boundary(&posts[1..], Some(1)).xpect_none();
	}

	/// Ann says a fact to change and one to delete, then filler, and the
	/// coach compacts them all away: answering the two posts.
	async fn facts_compacted(chat: &mut Chat) -> (Post, Post) {
		let original = chat.say(&ANN, "ORIGINAL-FACT the meeting is on monday");
		let deleted = chat.say(&ANN, "DELETED-FACT my password is hunter2");
		chat.said(posts(0..12));
		chat.turn(&COACH).await;
		chat.compactions(&COACH).len().xpect_eq(1);
		chat.compactions(&COACH)[0]
			.1
			.text
			.as_str()
			.xpect_contains("ORIGINAL-FACT")
			.xpect_contains("DELETED-FACT");
		(original, deleted)
	}

	/// An edit and a deletion of posts a summary covers make it stale; before
	/// the next call it is refreshed, both changes in one call, and no model
	/// input of the agent holds either original or the stale summary.
	#[beet_core::test]
	async fn a_change_under_a_summary_is_refreshed_before_the_next_call() {
		let mut chat = Chat::new(
			r#"Compact{template:Document("Scrub.bsx"), at:200, keep:60}"#,
		)
		.await;
		let (original, deleted) = facts_compacted(&mut chat).await;
		chat.edit(&ANN, original.id(), "NEW-FACT the meeting is on tuesday")
			.await;
		chat.delete(&ANN, deleted.id()).await;
		chat.records().compaction(&COACH).xpect_none();
		// a stale summary reaches no model
		for request in chat.requests(&COACH).await {
			request.as_str().xnot().xpect_contains("ORIGINAL-FACT");
		}
		chat.turn(&COACH).await;
		let compactions = chat.compactions(&COACH);
		compactions.len().xpect_eq(2);
		let (first, _) = &compactions[0];
		let (_, refreshed) = &compactions[1];
		refreshed.previous.xpect_eq(Some(first.id()));
		refreshed.from.xpect_eq(compactions[0].1.from);
		compactions[1].0.subject().xpect_eq(first.subject());
		refreshed
			.text
			.as_str()
			.xpect_contains("NEW-FACT")
			.xnot()
			.xpect_contains("ORIGINAL-FACT")
			.xnot()
			.xpect_contains("hunter2");
		for request in chat.requests(&COACH).await {
			request
				.as_str()
				.xpect_contains("NEW-FACT")
				.xnot()
				.xpect_contains("ORIGINAL-FACT")
				.xnot()
				.xpect_contains("hunter2");
		}
		// what was there is kept, in everything
		chat.records()
			.posts()
			.iter()
			.any(|post| post.to_string().contains("hunter2"))
			.xpect_true();
	}

	/// A refresh that fails ends the turn on its error, answering nothing
	/// over the stale summary.
	#[beet_core::test]
	async fn a_failed_refresh_answers_nothing() {
		let mut chat = Chat::new(
			r#"Compact{template:Document("Failing.bsx"), at:100000}"#,
		)
		.await;
		let original = chat.say(&ANN, "ORIGINAL-FACT");
		chat.compacted(&COACH, original.id(), "STALE-SUMMARY ORIGINAL-FACT");
		chat.edit(&ANN, original.id(), "NEW-FACT").await;
		chat.turn(&COACH).await;
		let last = chat.records().last_post().unwrap().clone();
		last.kind().clone().xpect_eq(PostKind::Error);
		last.to_string().as_str().xpect_contains("boom");
		chat.compactions(&COACH).len().xpect_eq(1);
		chat.records()
			.content_posts()
			.any(|view| view.post.to_string() == "ok")
			.xpect_false();
		chat.alone().xpect_true();
	}

	/// An interrupt drops a compaction in flight: the older one stays in
	/// force and no compaction thread is left behind.
	#[beet_core::test]
	async fn an_interrupted_compaction_leaves_the_old_one() {
		let mut chat = Chat::new(
			r#"Compact{template:Document("Held.bsx"), at:200, keep:60}"#,
		)
		.await;
		let first = chat.say(&ANN, "first");
		let older = chat.compacted(&COACH, first.id(), "OLDER-SUMMARY");
		chat.said(posts(0..12));
		let done = chat.spawn_turn(&COACH);
		chat.pump_until_held();
		let coach = chat.actor(&COACH);
		chat.app.world_mut().trigger(InterruptTurn::new(coach));
		chat.pump_until("ended the turn", |_| done.get());
		chat.pump_until("despawned the compaction thread", |world| {
			world
				.query_filtered::<(), With<Thread>>()
				.iter(world)
				.count() == 1
		});
		chat.compactions(&COACH).len().xpect_eq(1);
		chat.records()
			.compaction(&COACH)
			.map(Post::id)
			.xpect_eq(Some(older.id()));
	}

	/// A change landing while a compaction runs leaves that compaction
	/// stale, and a post typed meanwhile is read verbatim, nothing lost.
	#[beet_core::test]
	async fn what_lands_while_one_runs_is_not_lost() {
		let mut chat = Chat::new(
			r#"Compact{template:Document("Held.bsx"), at:200, keep:60}"#,
		)
		.await;
		let original = chat.say(&ANN, "ORIGINAL-FACT");
		chat.said(posts(0..12));
		let done = chat.spawn_turn(&COACH);
		let gate = chat.pump_until_held();
		chat.edit(&ANN, original.id(), "NEW-FACT").await;
		let typed = chat.say(&ANN, "TYPED-DURING");
		gate.release();
		chat.pump_until("ended the turn", |_| done.get());
		let (compaction, _) = chat.compactions(&COACH).remove(0);
		chat.records().is_stale(&compaction).xpect_true();
		let read = chat.records().meta(compaction.id()).unwrap().read.unwrap();
		(read < typed.id()).xpect_true();
		// the coach answered, having read what was typed
		chat.records()
			.content_posts()
			.any(|view| view.post.to_string() == "ok")
			.xpect_true();
		for request in chat.requests(&COACH).await {
			request.as_str().xpect_contains("TYPED-DURING");
		}
	}

	/// Two agents compact apart: each reads its own compaction, and one's
	/// bounds nothing of the other's context.
	#[beet_core::test]
	async fn two_agents_compact_apart() {
		let mentor = actor_id!("mentor");
		let mut chat = Chat::of(r#"<Thread {(Sequence, MountThreadStore{path:"chat"})}>
	<CreateActor name="Ann" kind="User"/>
	<CreateActor name="Coach" kind="Agent" {(MockPostStreamer{custom_response:"ok"}, Compact{template:Document("Fixed.bsx"), at:200, keep:60})}/>
	<CreateActor name="Mentor" kind="Agent" {(MockPostStreamer{custom_response:"ok"}, Compact{template:Document("Echo.bsx"), at:100000})}/>
</Thread>"#)
		.await;
		chat.said(posts(0..12));
		chat.turn(&COACH).await;
		chat.turn(&mentor).await;
		chat.compactions(&COACH).len().xpect_eq(1);
		chat.compactions(&mentor).xpect_empty();
		for request in chat.requests(&COACH).await {
			request
				.as_str()
				.xpect_contains("FIXED-SUMMARY")
				.xnot()
				.xpect_contains("POST-0 ");
		}
		for request in chat.requests(&mentor).await {
			request
				.as_str()
				.xnot()
				.xpect_contains("FIXED-SUMMARY")
				.xpect_contains("POST-0 ");
		}
	}

	/// The conversation [`Chat::new`] spawns, compacting through `template`.
	fn scene(template: &str) -> String {
		format!(
			r#"<Thread {{(Sequence, MountThreadStore{{path:"chat"}})}}>
	<CreateActor name="System" kind="System"><CreatePost text="SEED-RULE"/></CreateActor>
	<CreateActor name="Ann" kind="User"/>
	<CreateActor name="Coach" kind="Agent" {{(MockPostStreamer{{custom_response:"ok"}}, Compact{{template:Document("{template}"), at:200, keep:60}})}}/>
</Thread>"#
		)
	}

	/// A branch from before its parent's compaction reads its history whole,
	/// and one from after opens on the compaction it inherits.
	#[beet_core::test]
	async fn a_branch_inherits_a_compaction_only_after_it() {
		let scene = scene("Fixed.bsx");
		let mut chat = Chat::of(&scene).await;
		chat.said(posts(0..12));
		let before = chat.records().posts().iter().rev().nth(3).unwrap().id();
		chat.turn(&COACH).await;
		let after = chat.records().last_post().unwrap().id();
		chat.flush().await;
		let thread = chat.app.world().get::<Thread>(chat.window).unwrap().id();
		for (point, inherits) in [(before, false), (after, true)] {
			let branch = chat
				.enter_again(
					&scene,
					Request::get("chat").with_param(
						"branch-of",
						&PostRef::new(thread, point).to_string(),
					),
				)
				.await;
			let coach = chat.actor_in(branch, &COACH);
			for request in chat.requests_of(coach).await {
				request.contains("FIXED-SUMMARY").xpect_eq(inherits);
				request.contains("POST-0 ").xpect_eq(!inherits);
			}
		}
	}

	/// A window whose agents compact opens back through their compactions,
	/// the posts they cover left unread, and refreshing a stale one reads a
	/// changed post it lacks from the store.
	#[beet_core::test]
	async fn a_compacting_window_opens_back_through_its_compaction() {
		let scene = scene("Scrub.bsx");
		let mut chat = Chat::of(&scene).await;
		let (original, _) = facts_compacted(&mut chat).await;
		chat.edit(&ANN, original.id(), "NEW-FACT the meeting is on tuesday")
			.await;
		chat.flush().await;
		let old = chat.window;
		chat.app.world_mut().entity_mut(old).despawn();
		chat.window = chat.enter_again(&scene, Request::get("chat")).await;
		let records = chat.records();
		records.has_older().xpect_true();
		records.post(original.id()).xpect_none();
		records.post(records.seed().unwrap()).xpect_some();
		records.newest_compaction(&COACH).xpect_some();
		chat.turn(&COACH).await;
		let (_, refreshed) = chat.compactions(&COACH).pop().unwrap();
		refreshed
			.text
			.as_str()
			.xpect_contains("NEW-FACT")
			.xnot()
			.xpect_contains("ORIGINAL-FACT");
		for request in chat.requests(&COACH).await {
			request
				.as_str()
				.xpect_contains("SEED-RULE")
				.xpect_contains("NEW-FACT")
				.xnot()
				.xpect_contains("ORIGINAL-FACT");
		}
	}

	/// A small deterministic generator, so a failing shape replays.
	struct Shapes(u64);
	impl Shapes {
		fn next(&mut self, below: u64) -> u64 {
			self.0 = self
				.0
				.wrapping_mul(6364136223846793005)
				.wrapping_add(1442695040888963407);
			(self.0 >> 33) % below.max(1)
		}
	}

	/// Over random threads, no post is ever outside the seed, a summary and
	/// what is read verbatim: compactions leave no gap, and every shown post
	/// after the newest one's subject reaches the model.
	#[beet_core::test]
	async fn no_post_is_ever_missing() {
		for seed in 0..12u64 {
			let mut shapes = Shapes(seed);
			let compact = format!(
				r#"Compact{{template:Document("Scrub.bsx"), at:{}, keep:{}, {} seed:{}}}"#,
				100 + shapes.next(300),
				shapes.next(150),
				match shapes.next(2) {
					0 => String::new(),
					_ => format!("step:{},", 20 + shapes.next(100)),
				},
				["Keep", "Compact"][shapes.next(2) as usize],
			);
			let mut chat = Chat::new(&compact).await;
			let mut said = Vec::new();
			for _ in 0..8 {
				for _ in 0..shapes.next(5) {
					let words = "word ".repeat(shapes.next(30) as usize);
					said.push(
						chat.say(&ANN, &format!("SAID-{} {words}", said.len())),
					);
				}
				// the coach's own call answered, a note typed between
				if shapes.next(3) == 0 {
					let mut window =
						chat.app.world_mut().entity_mut(chat.window);
					let thread = window.get::<Thread>().unwrap().id();
					let call = AgentPost::new_function_call(
						COACH,
						thread,
						"search",
						"c1",
						"{}",
						PostStatus::Completed,
					);
					let note = AgentPost::new_text(
						ANN,
						thread,
						"NOTE",
						PostStatus::Completed,
					);
					let output = AgentPost::new_function_call_output(
						COACH,
						thread,
						call.id(),
						"c1",
						"found",
						PostStatus::Completed,
					);
					let mut records =
						window.get_mut::<ThreadRecords>().unwrap();
					for post in [call, note, output] {
						records.upsert_post(post);
					}
				}
				// take back something said
				if !said.is_empty() && shapes.next(3) == 0 {
					let post =
						said.remove(shapes.next(said.len() as u64) as usize);
					match shapes.next(2) {
						0 => chat.edit(&ANN, post.id(), "EDITED").await,
						_ => chat.delete(&ANN, post.id()).await,
					}
				}
				chat.turn(&COACH).await;
				let records = chat.records();
				assert_no_gap(records, &COACH);
				let shown = records.shown(..);
				let subject =
					records.compaction(&COACH).and_then(Post::subject);
				let read = records
					.model_posts(&shown, &COACH, None)
					.into_iter()
					.flat_map(|item| match item {
						ModelPost::Post(view) => vec![view.post.id()],
						ModelPost::Call { call, output } => [
							Some(call.post.id()),
							output.map(|output| output.post.id()),
						]
						.into_iter()
						.flatten()
						.collect(),
						ModelPost::Note(_) => Vec::new(),
					})
					.collect::<HashSet<_>>();
				for post in shown.iter().filter(|post| {
					subject.is_none_or(|subject| post.id() > subject)
						&& post
							.as_agent_post()
							.is_some_and(|post| post.is_model_input())
				}) {
					read.contains(&post.id()).xpect_true();
				}
				chat.alone().xpect_true();
			}
		}
	}
}
