//! Threads: conversations kept as records, and the windows that show and
//! run them.
//!
//! A thread is a conversation the way a group chat is one: people post to
//! each other and open sub-threads whether or not an agent is around, and an
//! agent is one more participant. Every word below holds for a thread with no
//! agent in it, and each is defined here once; every other page cites this
//! one.
//!
//! # Records
//!
//! - **Thread.** A conversation: a sequence of posts, kept as records in a
//!   store ([`Thread`], named by its [`ThreadId`]).
//! - **Actor.** A participant, a person, an agent or the system, all on equal
//!   footing ([`Actor`], its [`ActorKind`]). An actor is a record; it is an
//!   entity only as behavior, the action that takes its turn.
//! - **Post.** One event of a thread, authored by one actor ([`Post`]): a
//!   message, a reasoning step, a tool call, its output, or a log of
//!   something that happened. Its [`PostIntent`] is closed and a log's kind
//!   is open ([`LogKind`]), a downstream kind prefixing its crate.
//! - **Seed.** The posts a template opens a new thread with, authored as
//!   [`<CreatePost>`](CreatePost) children of its actors. It is written once,
//!   by the entry creating the thread, as the thread's first posts, and
//!   nothing in the store marks them: a stored thread has posts, not a seed.
//!   A template is built on every entry, so an entry continuing a thread
//!   discards its seed unread, and an edited seed reaches a new thread only
//!   (`--new`).
//!
//! Anything persisted names another thread or post by id, never by `Entity`.
//!
//! # The store
//!
//! The store path names the thread. `MountThreadStore{path:"threads/coach/{claim}"}`
//! fills each `{param}` from the request entering the thread, a greedy path
//! param as the path it captured joined with `/`, so a filled path may nest.
//! The directory is the key: the newest thread there continues, `--new`
//! starts another and `--thread=<id>` names one ([`ThreadEntryParams`]).
//! Nothing matches on metadata. A param value holding a `..` segment, a
//! leading `/` or nothing is refused, naming the param, so no request reaches
//! outside the mount's own prefix. A mount declaring `new:true` persists and
//! never continues: every entry behaves as `--new`.
//!
//! The store is the `TableStore` at the mount path: the thread, each actor,
//! each post and each response's metadata ([`ResponseMeta`]) is its own row,
//! keyed by its id. A write is a row, never a rewrite of the thread, so two
//! writers on one thread never erase each other, and whatever backs the
//! table, json over blobs, sqlite or dynamo, backs the thread.
//!
//! # Windows
//!
//! - **Window.** A thread entity, open onto one stored thread, which is the
//!   truth: it holds that thread's records in memory ([`ThreadRecords`], loaded whole), writes what it posts to the
//!   store and receives what other windows post. A thread has any number of
//!   windows, on any surfaces, each with its own view and composer.
//! - **Feed.** Within one runtime, a post pushed to one window's records is
//!   applied to every other window of the same stored thread at once, a post
//!   in progress included, so a reply streams in every window. Only a
//!   completed post is written to the store, once, by the window that
//!   authored it; a window never writes a post it received.
//! - **Behavior.** A scene holding an action on an actor, ie a `Sequence`
//!   over an agent's streamer and a person's `UserInput`.
//! - **Canonical window.** The window holding a thread's behavior; a thread
//!   has at most one in a runtime, and every other window is a view and a
//!   composer with no behavior. Entering a thread with behavior while another
//!   window holds behavior for it is refused, naming both: two agents
//!   answering is a mistake in what was installed, never a race to resolve.
//!   A post from any window reaches the canonical window by the feed, so a
//!   person typing in a browser tab is answered, or interrupts, exactly as one
//!   typing beside it.
//!
//! A window lives as long as the scene holding it. `RunThread` holds a
//! canonical window for the runtime's life, entered with the start request,
//! which is how a thread a person sits in is held.
//!
//! - **Thread route.** A thread run as a function (`ThreadRoute`): built for
//!   one request, run to its outcome, answered and despawned, nobody typing.
//!   It refuses a template holding a `UserInput`, and a request for a thread
//!   whose canonical window is live answers `409`.
//!
//! # Entering
//!
//! `Thread::enter` is the one place that knows whether a thread is created or
//! continued, so it owns everything that depends on the answer, in order:
//!
//! 1. fill the mount's path from the request, refusing an escaping value;
//! 2. find the stored thread (`--thread`, `--new`, the mount's `new`, else the
//!    newest in the directory), refusing a second canonical window;
//! 3. creating: record `spawned_by` and `branch_of`, and for a fresh thread
//!    resolve each seed source (`<CreatePost src="..">`, a route's answer)
//!    in author order and keep the scene's seed posts;
//! 4. continuing, or branching, which continues its parent's history: drop
//!    the scene's seed posts, reading no seed source and calling no seed
//!    route;
//! 5. load the stored records, join the feed, post a join for each actor
//!    present that no other window of the runtime holds, then post the
//!    request's message if the entry carries one.
//!
//! A seed source is resolved by the entry creating its thread and by nothing
//! else, so building a template reads no file and calls no route.
//!
//! A request's message belongs to the request:
//! `ThreadRoute{template:"..", message:"review {document}", actor:2}`, each
//! `{param}` filled from the request and posted after the joins, on every
//! request. Without `actor` its author is the thread's first actor of kind
//! User by id, and a thread holding none fails the request.
//!
//! # Sessions
//!
//! - **Session.** An actor's stay in a thread, from its join to its last post
//!   before its next join. A session belongs to an actor, never to a window
//!   or a runtime, so several people, and agents that come and go, each have
//!   their own.
//! - **Join.** A [`SessionOpened`] post authored by the actor joining, the
//!   only session post, carrying the entry's `spawned_by`. Entering joins
//!   every actor the scene authors that is not of kind System; a System actor
//!   authors seeds and never joins, and nothing creates one on a template's
//!   behalf. Nothing is written when an actor leaves or a runtime stops, so
//!   nothing depends on a graceful exit. Usage stays where it is measured, on
//!   each response's [`ResponseMeta`], and a session's duration is the span
//!   of its posts.
//! - **Note.** How a model reads a join, one convention for every provider:
//!   `<system>Coach joined at 2026-11-22T09:10Z, 2 days 3 hours after the last post.</system>`.
//!   The gap is measured from the last post that is not a log, and omitted
//!   when there is none or it is under a minute. Consecutive joins are one
//!   message, a line each inside one `<system>`. A note is sent in the user
//!   role whoever joined, the reading agent's own join included: it is
//!   something that happened, never something the agent said.
//!
//! # Posting and turns
//!
//! The records are the bus. Any actor present may post at any time, and
//! posting never requires holding a turn: a composer appends its post the
//! moment it is submitted.
//!
//! - **Turn.** An action. A thread's behavior decides when each actor's
//!   action runs, and that is the whole of turn taking; a thread with no
//!   behavior is a room people post in. A person's turn (`UserInput`) is a
//!   wait on the records, passing once the newest post that is not a log is
//!   its actor's.
//! - **Interrupt.** A running turn cut short (`InterruptTurn`), beet's own and
//!   the same for every provider, since no request API takes input during a
//!   response: the model's stream is dropped, the post in progress closed as
//!   interrupted, the turn's tool calls cancelled, each given an output
//!   saying so, and the turn passes. `InterruptOnPost` on an agent is that
//!   agent's manners: any post by another actor interrupts it.
//!
//! A thread has one of three **shapes**, each the shape of its behavior and
//! its actors, never a mode of the thread, and each independent of whether an
//! actor speaks or types:
//!
//! - **Turn based.** A `Sequence` of actors, one acting at a time. A post made
//!   while an agent works stops nothing and is not lost: every model call
//!   reads the whole records, so the agent reads it at its next call, ie
//!   between two tool calls of the same turn.
//! - **Semi duplex.** Turn based with interrupts: its agents carry
//!   `InterruptOnPost`, so a post, or the composer's stop control, cuts the
//!   running turn and the `Sequence` moves on, answering the person now. It
//!   is the same `Sequence`, since a person's turn already passes when the
//!   last word is theirs.
//! - **Full duplex.** Actors under a `Parallel` act at once and may speak over
//!   each other. Nothing interrupts anybody on another's behalf: whether an
//!   actor yields when another speaks is its own `InterruptOnPost`.
//!
//! # Tools
//!
//! - **Tool.** A route of an agent's own `<Router>`, its identity its route
//!   path, so a call of anything else is that router's not-found. A route of
//!   the surrounding url space becomes a tool through `<RouteTool/>`.
//!
//! # Lineage
//!
//! Two relations between threads live on the [`Thread`] record, each naming a
//! post by [`PostRef`] (`<thread id>/<post id>`) and each set by an entry
//! param:
//!
//! - **Spawned** (`--spawned-by`): started by the action of a post in another
//!   thread, ie a delegate call, a loop's decision or a person opening a
//!   sub-thread from a post. Causal only, no context comes with it.
//! - **Branch** (`--branch-of`): a thread continuing another's history up to
//!   a post, then going its own way, referencing its ancestors' posts rather
//!   than copying them. A branch lives in its parent's store, so
//!   `--branch-of` naming a post the mount's store lacks is refused, as is
//!   `--branch-of` beside `--thread`. It opens with its parent's posts, so
//!   creating a branch reads no seed.
//!
//! # Limits
//!
//! Recorded here and built by nothing:
//!
//! - every surface of a template is the same User actor, since people have no
//!   identity yet;
//! - no route lists the threads of a store;
//! - the feed is one runtime's, so windows in two runtimes see each other's
//!   posts only on entering, and a backend's own change feed is unused;
//! - an actor's leaving is not recorded: presence is a status nothing keeps;
//! - compaction: nothing bounds a thread by post count, and fitting a long
//!   thread to a model's context is not built.
mod actor;
mod create_actor;
pub use create_actor::*;
mod create_post;
mod document;
mod load_thread;
pub use create_post::*;
pub use load_thread::*;
#[cfg(feature = "action")]
mod route_post;
#[cfg(feature = "action")]
pub use route_post::*;
#[cfg(feature = "action")]
mod seed_source;
mod thread;
mod thread_query;
mod thread_records;
pub mod thread_store;
#[cfg(feature = "action")]
pub use seed_source::*;
pub use thread_store::SyncedPosts;
mod blob_thread_store;
mod thread_store_provider;
pub use blob_thread_store::*;
pub use document::*;
pub use thread_store_provider::*;
mod agent_post;
pub use agent_post::*;
mod post;
mod post_log;
mod post_ref;
pub use post_ref::*;
mod session;
pub use actor::*;
pub use post::*;
pub use post_log::*;
pub use session::*;
pub use thread::*;
pub use thread_query::*;
pub use thread_records::*;
mod response_meta;
pub use response_meta::*;
#[cfg(feature = "action")]
mod stub_old_images;
#[cfg(feature = "action")]
pub use stub_old_images::*;
