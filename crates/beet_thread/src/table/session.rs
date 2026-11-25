//! Sessions, the words defined in the [`table`](crate::table) module docs.
//!
//! A session is an actor's stay in a thread, from its join to its last post
//! before its next join. Entering a window appends a [`SessionOpened`] post,
//! a join, for each actor its scene authors that no other window of the
//! runtime holds and that is not of kind System, authored by that actor. It
//! is the only session post: nothing is written when an actor leaves.
//!
//! A model reads a run of joins as one note ([`ThreadRecords::join_note`],
//! placed by [`ThreadRecords::model_posts`]) and a `ThreadView` renders each
//! join as a divider naming its actor.
use crate::prelude::*;
use beet_core::prelude::*;

/// The log of an actor joining its thread, authored by the actor.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionOpened {
	/// The post whose action asked for the entry, ie the loop decision
	/// requesting the unit. A thread keeps its creator in
	/// [`Thread::spawned_by`]; each join records its entry's cause here.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub spawned_by: Option<PostRef>,
}

impl LogKind for SessionOpened {
	const KIND: &'static str = "session_opened";
}

impl SessionOpened {
	/// Whether `post` is a join.
	pub fn opens(post: &Post) -> bool {
		post.log_kind().as_deref() == Some(Self::KIND)
	}
}

impl ThreadRecords {
	/// Append a [`SessionOpened`] post for each of `actors` that is not of
	/// kind System, authored by it.
	///
	/// ## Errors
	///
	/// Errors when an actor is not in the records.
	pub fn join(
		&mut self,
		thread: ThreadId,
		actors: impl IntoIterator<Item = ActorId>,
		opened: &SessionOpened,
	) -> Result {
		for actor in actors {
			if self.actor(actor)?.kind() == ActorKind::System {
				continue;
			}
			self.upsert_post(Post::new_log(actor, thread, opened)?);
		}
		Ok(())
	}

	/// The note a model reads the run of consecutive `joins` as, the one
	/// convention for every provider: a line for each join inside one
	/// `<system>`, saying who joined, when, and how long after the last post.
	///
	/// ```text
	/// <system>Coach joined at 2026-11-22T09:10Z, 2 days 3 hours after the last post.</system>
	/// ```
	///
	/// The gap is measured from the last post before the run that is not a
	/// log, and omitted when there is none or it is under a minute.
	pub fn join_note(&self, joins: &[AuthoredPost]) -> String {
		let last_said = joins.first().and_then(|first| {
			self.posts()
				.iter()
				.take_while(|post| post.id() != first.post.id())
				.filter(|post| post.intent() != PostIntent::Log)
				.last()
				.map(Post::created)
		});
		let lines = joins
			.iter()
			.map(|join| {
				let joined = join.post.created();
				let gap = last_said
					.and_then(|said| gap_text(joined.millis() - said.millis()))
					.map(|gap| format!(", {gap} after the last post"))
					.unwrap_or_default();
				format!(
					"{} joined at {}{gap}.",
					join.actor.name(),
					joined.format_iso8601_mins()
				)
			})
			.collect::<Vec<_>>()
			.join("\n");
		format!("<system>{lines}</system>")
	}
}

/// A gap of `millis` in its two largest units, ie `2 days 3 hours`, the
/// second omitted when zero. None under a minute.
fn gap_text(millis: i64) -> Option<String> {
	let minutes = millis / 60_000;
	let units = [
		(minutes / (24 * 60), "day"),
		(minutes / 60 % 24, "hour"),
		(minutes % 60, "minute"),
	];
	let first = units.iter().position(|(count, _)| *count > 0)?;
	units[first..]
		.iter()
		.take(2)
		.filter(|(count, _)| *count > 0)
		.map(|(count, unit)| match count {
			1 => format!("1 {unit}"),
			_ => format!("{count} {unit}s"),
		})
		.collect::<Vec<_>>()
		.join(" ")
		.xsome()
}

#[cfg(all(test, feature = "agent"))]
mod test {
	use super::*;
	use beet_net::prelude::*;

	const MINUTE: i64 = 60_000;
	const HOUR: i64 = 60 * MINUTE;
	const DAY: i64 = 24 * HOUR;
	/// How long ago the stored thread of [`away`] was last posted in.
	const AWAY: i64 = 2 * DAY + 3 * HOUR + MINUTE;
	const PERSON: u128 = 1;
	const COACH: u128 = 2;
	const EDITOR: u128 = 3;

	#[beet_core::test]
	fn gaps_read_in_two_units() {
		gap_text(MINUTE - 1).xpect_none();
		gap_text(-DAY).xpect_none();
		gap_text(MINUTE).unwrap().xpect_eq("1 minute");
		gap_text(HOUR + 5 * MINUTE)
			.unwrap()
			.xpect_eq("1 hour 5 minutes");
		gap_text(2 * DAY + 3 * HOUR + 59 * MINUTE)
			.unwrap()
			.xpect_eq("2 days 3 hours");
		// a zero second unit is dropped, never skipped over
		gap_text(2 * DAY + 5 * MINUTE).unwrap().xpect_eq("2 days");
	}

	fn actor(id: u128) -> Actor {
		let (name, kind) = match id {
			PERSON => ("Person", ActorKind::User),
			COACH => ("Coach", ActorKind::Agent),
			_ => ("Editor", ActorKind::User),
		};
		Actor::new_with_id(ActorId::from_u128(id), name, kind)
	}

	/// `post`, created `millis` earlier.
	fn aged(post: Post, millis: i64) -> Post {
		let mut value = serde_json::to_value(&post).unwrap();
		value["created"] = (post.created().millis() - millis).into();
		serde_json::from_value(value).unwrap()
	}

	/// An app over a store.
	fn app() -> (App, Entity) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_types::<ThreadPlugin>();
		let root = app.world_mut().spawn(InMemoryStore::new()).id();
		(app, root)
	}

	/// Store a thread the person and the coach joined [`AWAY`] ago, the
	/// person posting once, and nothing since.
	async fn away(app: &App, root: Entity) {
		let thread = Thread::default();
		let actors = [PERSON, COACH].map(actor).to_vec();
		let mut posts = actors
			.iter()
			.map(|actor| {
				Post::new_log(
					actor.id(),
					thread.id(),
					&SessionOpened::default(),
				)
				.unwrap()
			})
			.collect::<Vec<_>>();
		posts.push(AgentPost::new_text(
			ActorId::from_u128(PERSON),
			thread.id(),
			"see you",
			PostStatus::Completed,
		));
		ThreadStore::new(
			app.world().get::<TableStore>(root).unwrap().clone(),
			RelPath::new("threads"),
		)
		.insert(
			&thread,
			actors,
			posts.into_iter().map(|post| aged(post, AWAY)).collect(),
			default(),
		)
		.await
		.unwrap();
	}

	/// Enter a window of `actors` mounting the store at `root`, each agent a
	/// mock, answering the window and the coach's behavior entity.
	async fn enter(
		app: &mut App,
		root: Entity,
		actors: &[u128],
	) -> (Entity, Entity) {
		let mut window = app.world_mut().spawn((
			ChildOf(root),
			Thread::default(),
			MountThreadStore {
				path: RelPath::new("threads"),
				new: false,
			},
		));
		for actor in actors.iter().copied().map(actor) {
			match actor.kind() {
				ActorKind::Agent => {
					window.with_child((actor, MockPostStreamer::default()))
				}
				_ => window.with_child(actor),
			};
		}
		let window = window.id();
		app.world_mut()
			.run_async_local_then(async move |world: AsyncWorld| -> Result {
				Thread::enter(
					world.clone(),
					window,
					ThreadEntry::parse(Request::get("chat").parts().clone())?,
				)
				.await
			})
			.await
			.unwrap();
		let coach = app
			.world_mut()
			.query::<(Entity, &ActorRef)>()
			.iter(app.world())
			.find(|(_, actor)| ***actor == ActorId::from_u128(COACH))
			.unwrap()
			.0;
		(window, coach)
	}

	fn model() -> ModelDef {
		ModelDef {
			provider_slug: "provider".into(),
			model_slug: "model".into(),
			url: "http://localhost".into(),
			auth: None,
		}
	}

	/// The `(role, content)` of each message under `key` of a request.
	fn messages(request: &impl Serialize, key: &str) -> Vec<(String, String)> {
		serde_json::to_value(request).unwrap()[key]
			.as_array()
			.unwrap()
			.iter()
			.map(|message| {
				(
					message["role"].as_str().unwrap().to_string(),
					message["content"].as_str().unwrap().to_string(),
				)
			})
			.collect()
	}

	/// The request `streamer` builds for the `coach`.
	async fn responses_request(
		app: &mut App,
		coach: Entity,
		streamer: O11sStreamer,
	) -> o11s::RequestBody {
		app.world_mut()
			.run_async_local_then(async move |world: AsyncWorld| {
				streamer.build_request(world.entity(coach)).await
			})
			.await
			.unwrap()
			.0
	}

	/// The `(role, content)` of each message the coach's model reads, the
	/// same by the responses protocol and by completions.
	async fn model_reads(
		app: &mut App,
		coach: Entity,
	) -> Vec<(String, String)> {
		let responses =
			responses_request(app, coach, O11sStreamer::new(model()))
				.await
				.xmap(|request| messages(&request, "input"));
		let completions = app
			.world_mut()
			.run_async_local_then(async move |world: AsyncWorld| {
				CompletionsStreamer::new(model())
					.build_request(world.entity(coach))
					.await
			})
			.await
			.unwrap()
			.0;
		messages(&completions, "messages").xpect_eq(responses.clone());
		responses
	}

	/// The lines of the one note `message` is, sent in the user role.
	fn note_lines((role, content): &(String, String)) -> Vec<String> {
		role.xpect_eq("user");
		content
			.strip_prefix("<system>")
			.and_then(|content| content.strip_suffix("</system>"))
			.unwrap()
			.lines()
			.map(str::to_string)
			.collect()
	}

	/// A thread continued after a gap gives the model who joined and how long
	/// after, with no post marking the earlier session's end.
	#[beet_core::test]
	async fn a_continued_thread_reads_who_came_back() {
		let (mut app, root) = app();
		away(&app, root).await;
		let (window, coach) = enter(&mut app, root, &[PERSON, COACH]).await;
		let records = app.world().get::<ThreadRecords>(window).unwrap();
		// only joins and the person's post: nothing closed the first session
		records
			.posts()
			.iter()
			.map(SessionOpened::opens)
			.collect::<Vec<_>>()
			.xpect_eq(vec![true, true, false, true, true]);
		let joined =
			records.last_post().unwrap().created().format_iso8601_mins();
		let reads = model_reads(&mut app, coach).await;
		reads.len().xpect_eq(3);
		// the first joins, with no post before them to measure from
		let first = note_lines(&reads[0]);
		first.len().xpect_eq(2);
		first[0].as_str().xpect_starts_with("Person joined at ");
		first[0]
			.as_str()
			.xnot()
			.xpect_contains("after the last post");
		reads[1].1.as_str().xpect_contains("see you");
		note_lines(&reads[2]).xpect_eq(vec![
			format!(
				"Person joined at {joined}, 2 days 3 hours after the last post."
			),
			format!(
				"Coach joined at {joined}, 2 days 3 hours after the last post."
			),
		]);
	}

	/// Three actors joining at once reach the model as one message, the
	/// reading agent's own join among them, in the user role.
	#[beet_core::test]
	async fn joins_at_once_are_one_message() {
		let (mut app, root) = app();
		let (_, coach) = enter(&mut app, root, &[PERSON, COACH, EDITOR]).await;
		let reads = model_reads(&mut app, coach).await;
		reads.len().xpect_eq(1);
		note_lines(&reads[0])
			.iter()
			.map(|line| line.split(' ').next().unwrap().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["Person", "Coach", "Editor"]);
	}

	/// A thread whose only post is an agent's join still makes a valid first
	/// request: one user message.
	#[beet_core::test]
	async fn a_lone_join_is_a_valid_request() {
		let (mut app, root) = app();
		let (_, coach) = enter(&mut app, root, &[COACH]).await;
		let reads = model_reads(&mut app, coach).await;
		reads.len().xpect_eq(1);
		note_lines(&reads[0]).len().xpect_eq(1);
	}

	/// A response chained by `previous_response_id` receives each note once:
	/// the joins since the stored response and none before it.
	#[beet_core::test]
	async fn a_chained_response_reads_each_note_once() {
		let (mut app, root) = app();
		let (window, coach) = enter(&mut app, root, &[PERSON, COACH]).await;
		let streamer =
			|| O11sStreamer::new(model()).with_use_previous_response_id();
		// nothing stored yet, so the first request carries the first joins
		let first = responses_request(&mut app, coach, streamer()).await;
		messages(&first, "input").len().xpect_eq(1);
		first.previous_response_id.xpect_none();

		// the coach answers, its response stored, then the person comes back
		let answer = |app: &mut App, response: &str| {
			let mut window = app.world_mut().entity_mut(window);
			let thread = window.get::<Thread>().unwrap().id();
			let post = AgentPost::new_text(
				ActorId::from_u128(COACH),
				thread,
				"hello",
				PostStatus::Completed,
			);
			let mut records = window.get_mut::<ThreadRecords>().unwrap();
			records.set_meta(ResponseMeta {
				post_id: post.id(),
				provider_slug: "provider".into(),
				model_slug: "model".into(),
				response_id: response.into(),
				response_stored: true,
				usage: None,
			});
			records.upsert_post(post);
			thread
		};
		let thread = answer(&mut app, "response-1");
		app.world_mut()
			.get_mut::<ThreadRecords>(window)
			.unwrap()
			.join(thread, [ActorId::from_u128(PERSON)], &default())
			.unwrap();
		let second = responses_request(&mut app, coach, streamer()).await;
		second
			.previous_response_id
			.as_deref()
			.xpect_eq(Some("response-1"));
		let reads = messages(&second, "input");
		reads.len().xpect_eq(1);
		note_lines(&reads[0]).len().xpect_eq(1);

		// answered again, the note is behind the stored response
		answer(&mut app, "response-2");
		let third = responses_request(&mut app, coach, streamer()).await;
		messages(&third, "input").xpect_empty();
	}
}
