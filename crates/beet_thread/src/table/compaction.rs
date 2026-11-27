//! Compactions, the words defined in the [`table`](crate::table) module docs
//! under "Contexts and compaction": what a `compaction` post holds, when one
//! may be read, and the context an agent reads through it. Making one is
//! [`Compact`](crate::prelude::Compact)'s.
use crate::prelude::*;
use beet_core::prelude::*;

/// The body of a `compaction` post, a stored summary of everything up to its
/// subject, authored by the agent whose context it is.
///
/// What it covers is its own run, `from` through its subject, and, when it
/// folded in a `previous` compaction, everything that one covers: a chain
/// back to the first compaction, whose `from` is the thread's first post
/// unless `backfill` cut it, in which case its text says where it starts.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Compaction {
	/// The summary.
	pub text: String,
	/// The first post the compaction thread was given verbatim, `None` for a
	/// refresh, which is given no posts.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub from: Option<PostId>,
	/// The compaction folded in, whose summary this one continues.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub previous: Option<PostId>,
	/// Whether the seed is inside the summary rather than held apart.
	#[serde(default)]
	pub seed: bool,
}

impl LogKind for Compaction {
	const KIND: &'static str = "compaction";
}

impl Compaction {
	/// One post as a compaction thread reads it, with its id, author and
	/// time, ie `<post id="3mw.." author="Ann" at="2026-11-20T09:10Z">hi</post>`:
	/// a join as `(joined)`, and with `everything` a deleted post's body
	/// and an edit's original. `None` for what no model reads, reasoning, a
	/// compaction and any other log.
	pub fn post_line(
		post: &Post,
		author: &str,
		everything: bool,
	) -> Option<String> {
		let text = match post.kind() {
			PostKind::SessionOpened => "(joined)".to_string(),
			PostKind::Edited if everything => {
				let was = post
					.log::<PostEdited>()
					.map(|edited| {
						edited
							.was
							.get("text")
							.ok()
							.and_then(|text| text.as_str().ok())
							.map(str::to_string)
							.unwrap_or_else(|| {
								Value::Map(edited.was).to_json_string()
							})
					})
					.unwrap_or_default();
				format!(
					"(edited post {}, which said: {was})",
					post.subject().map(|id| id.to_string()).unwrap_or_default()
				)
			}
			PostKind::Deleted if everything => format!(
				"(deleted post {})",
				post.subject().map(|id| id.to_string()).unwrap_or_default()
			),
			kind if kind.is_log() => return None,
			PostKind::Reasoning | PostKind::ReasoningSummary => return None,
			_ if everything && post.is_deleted() => {
				let mut said = post.clone();
				said.set_status(PostStatus::Completed);
				format!("(deleted) {}", Self::post_text(&said))
			}
			_ => Self::post_text(post),
		};
		format!(
			"<post id=\"{}\" author=\"{author}\" at=\"{}\">{text}</post>",
			post.id(),
			post.created().format_iso8601_mins()
		)
		.xsome()
	}

	/// The text of a post as a compaction thread reads it: what was said, a
	/// call as what it called, an output with each file's listing, a file
	/// its listing, so a reader names each by its post.
	pub fn post_text(post: &Post) -> String {
		match post.as_agent_post() {
			Some(AgentPost::Deleted(_)) => "(deleted)".to_string(),
			Some(AgentPost::FunctionCall(call)) => {
				format!("called `{}` with {}", call.name(), call.arguments())
			}
			Some(AgentPost::FunctionCallOutput(output)) => {
				[output.output().to_string()]
					.into_iter()
					.filter(|text| !text.is_empty())
					.chain(output.files().iter().map(PostFile::listing))
					.collect::<Vec<_>>()
					.join("\n")
			}
			Some(AgentPost::Media(_)) => post
				.files()
				.iter()
				.map(PostFile::listing)
				.collect::<Vec<_>>()
				.join("\n"),
			_ => post.to_string(),
		}
	}
}

impl ThreadRecords {
	/// How many tokens `post` costs a context, the one estimate used
	/// everywhere a count is not measured: a quarter of its row's length as
	/// json, rounded up, and a thousand for each file it holds.
	pub fn estimate(post: &Post) -> u64 {
		let row = Value::Map(post.body().clone()).to_json_string().len() as u64;
		row.div_ceil(4) + 1000 * post.files().len() as u64
	}

	/// The newest compaction `author` wrote, stale or not.
	pub fn newest_compaction(&self, author: &ActorId) -> Option<&Post> {
		self.compactions()
			.rev()
			.find(|(_, by)| *by == author)
			.and_then(|(id, _)| self.post(id))
	}

	/// Whether `compaction` is stale: an `edited` or `deleted` log newer
	/// than the newest post its message was built at (its
	/// [`ResponseMeta::read`]) names a post it covers, a branch's inherited
	/// logs included. A stale compaction is never read.
	pub fn is_stale(&self, compaction: &Post) -> bool {
		!self.changes_under(compaction).is_empty()
	}

	/// The posts changed under `compaction` since its message was built,
	/// each once, with the first log changing it since, in post order: a
	/// change to a post it does not cover is no change to it.
	pub fn changes_under(&self, compaction: &Post) -> Vec<(PostId, PostId)> {
		let Some(subject) = compaction.subject() else {
			return Vec::new();
		};
		let read = self
			.meta(compaction.id())
			.and_then(|meta| meta.read)
			.unwrap_or(compaction.id());
		let mut changed = Vec::<(PostId, PostId)>::new();
		for (log, changed_post) in self.changes_after(read) {
			if changed_post <= subject
				&& !changed.iter().any(|(post, _)| *post == changed_post)
			{
				changed.push((changed_post, log));
			}
		}
		changed.sort();
		changed
	}

	/// The compaction the context of `reader` opens with: its newest, unless
	/// stale, which nothing reads until it is refreshed.
	pub fn compaction(&self, reader: &ActorId) -> Option<&Post> {
		self.newest_compaction(reader)
			.filter(|compaction| !self.is_stale(compaction))
	}

	/// Every compaction that may be read, each author's newest unless stale:
	/// what [`Self::shown`] leaves the text of.
	pub(crate) fn readable_compactions(&self) -> HashSet<PostId> {
		let mut seen = HashSet::<&ActorId>::default();
		let mut readable = HashSet::default();
		for (id, author) in self.compactions().rev() {
			if seen.insert(author)
				&& let Some(post) = self.post(id)
				&& !self.is_stale(post)
			{
				readable.insert(id);
			}
		}
		readable
	}

	/// The bounds of the context of `reader` within `shown`: the newest
	/// seed post it reads apart from the summary, and the compaction it
	/// opens with, `None` for a context of the whole shown thread. A
	/// compaction whose text `shown` emptied, being stale, bounds nothing.
	pub fn context_bounds<'a>(
		&self,
		shown: &'a ShownPosts,
		reader: &ActorId,
	) -> Option<(Option<PostId>, &'a Post)> {
		let compaction = self.compaction(reader)?;
		let compaction = shown.post(compaction.id())?;
		let body = compaction.log::<Compaction>()?;
		let seed = (!body.seed).then_some(self.seed()).flatten();
		(seed, compaction).xsome()
	}

	/// Whether `post` is in the context bounded by `bounds`
	/// ([`Self::context_bounds`]): a seed post read apart, or a post after the
	/// compaction's subject.
	pub(crate) fn in_context(
		post: &Post,
		bounds: Option<(Option<PostId>, &Post)>,
	) -> bool {
		match bounds {
			None => true,
			Some((seed, compaction)) => {
				seed.is_some_and(|seed| post.id() <= seed)
					|| compaction
						.subject()
						.is_none_or(|subject| post.id() > subject)
			}
		}
	}

	/// The note a compaction is read as, opening its author's context: the
	/// summary, and through when it stands in for the posts.
	pub fn compaction_note(compaction: &Post) -> String {
		let text = compaction
			.log::<Compaction>()
			.map(|body| body.text)
			.unwrap_or_default();
		let through = compaction
			.subject()
			.map(|subject| {
				format!(" up to {}", subject.created().format_iso8601_mins())
			})
			.unwrap_or_default();
		format!(
			"<system>A summary of the conversation{through}, standing in for its posts:\n{text}</system>"
		)
	}

	/// How many tokens the context of `reader` holds: the last response's
	/// recorded input, if it was made after the newest compaction, and the
	/// estimate ([`Self::estimate`]) of what followed what it read, else the
	/// estimate of the whole context.
	pub fn context_tokens(&self, shown: &ShownPosts, reader: &ActorId) -> u64 {
		self.context_tokens_through(
			shown,
			reader,
			self.context_bounds(shown, reader),
		)
	}

	/// As [`Self::context_tokens`], the context bounded by `bounds`, ie a
	/// stale compaction standing as it was until it is refreshed.
	pub fn context_tokens_through(
		&self,
		shown: &ShownPosts,
		reader: &ActorId,
		bounds: Option<(Option<PostId>, &Post)>,
	) -> u64 {
		let floor = self.newest_compaction(reader).map(Post::id);
		let measured = self
			.posts()
			.iter()
			.rev()
			.take_while(|post| floor.is_none_or(|floor| post.id() > floor))
			.filter(|post| post.author() == reader && !post.is_log())
			.find_map(|post| {
				let meta = self.meta(post.id())?;
				(meta.usage?.input_tokens as u64, meta.read).xsome()
			});
		let (base, after) = match measured {
			Some((input, read)) => (input, read),
			None => (
				bounds
					.map(|(_, compaction)| {
						Self::compaction_note(compaction).len().div_ceil(4)
							as u64
					})
					.unwrap_or_default(),
				None,
			),
		};
		base + shown
			.iter()
			.filter(|post| after.is_none_or(|after| post.id() > after))
			.filter(|post| post.kind() != &PostKind::Compaction)
			.filter(|post| Self::in_context(post, bounds))
			.map(|post| Self::estimate(post))
			.sum::<u64>()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A compaction is stale once a post it covers changes after its message
	/// was built, and not for a change to a post it does not cover.
	#[beet_core::test]
	fn a_change_under_a_compaction_makes_it_stale() {
		let thread = ThreadId::now();
		let mut records = ThreadRecords::new();
		records.insert_actor(Actor::user());
		records.insert_actor(Actor::agent());
		let said = |text: &str| {
			AgentPost::new_text(
				ActorId::USER,
				thread,
				text,
				PostStatus::Completed,
			)
		};
		let covered = said("covered");
		let after = said("after");
		records.upsert_post(covered.clone());
		let compaction = Post::new_log(ActorId::AGENT, thread, &Compaction {
			text: "summary".into(),
			from: Some(covered.id()),
			..default()
		})
		.unwrap()
		.with_subject(covered.id());
		records.set_meta(ResponseMeta {
			post_id: compaction.id(),
			read: Some(covered.id()),
			..default()
		});
		records.upsert_post(compaction.clone());
		records.upsert_post(after.clone());
		records.compaction(&ActorId::AGENT).xpect_some();
		let edit = |post: &Post| {
			Post::new_log(ActorId::USER, thread, &PostEdited {
				was: post.body().clone(),
			})
			.unwrap()
			.with_subject(post.id())
		};
		// a post after its subject is read as it stands, no change to it
		records.upsert_post(edit(&after));
		records.compaction(&ActorId::AGENT).xpect_some();
		records.upsert_post(edit(&covered));
		records.compaction(&ActorId::AGENT).xpect_none();
		records.changes_under(&compaction).len().xpect_eq(1);
		// a stale compaction's text is shown to nobody
		records
			.shown(..)
			.post(compaction.id())
			.unwrap()
			.body()
			.is_empty()
			.xpect_true();
	}
}
