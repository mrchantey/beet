//! Threads: conversations kept as rows, and the windows that show and run
//! them.
//!
//! A thread is a conversation the way a group chat is one: people post to
//! each other and open sub-threads whether or not an agent is around, and an
//! agent is one more participant. Every word below holds for a thread with no
//! agent in it, and each is defined here once; every other page cites this
//! one.
//!
//! # Records
//!
//! - **Thread.** A conversation: a sequence of posts, kept as rows in a table
//!   store ([`Thread`], named by its [`ThreadId`]).
//! - **Actor.** A participant, a person, an agent or the system, all on equal
//!   footing ([`Actor`], its [`ActorKind`]). An actor is a record; it is an
//!   entity only as behavior, the action that takes its turn.
//! - **Post.** One event of a thread, authored by one actor ([`Post`]): a
//!   message, a reasoning step, a tool call, its output, or a log of
//!   something that happened, which is its kind (Posts, below).
//! - **Seed.** The posts a template opens a new thread with, authored as
//!   [`<CreatePost>`](CreatePost) children of its actors. It is written once,
//!   by the entry creating the thread, as the thread's first posts, and the
//!   thread's row names the last of them ([`Thread::seed`]); nothing else
//!   marks them. A template
//!   is built on every entry, so an entry continuing a thread discards its
//!   seed unread, and an edited seed reaches a new thread only (`--new`).
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
//! Tables and blobs are separate stores and neither implies the other. A
//! thread's rows live in the table store declared at or above it, which a
//! mount refuses without atomic batches and indexes, naming it and the fix,
//! as it refuses a missing one, never falling back. `JsonTableStore` is the
//! naive one, for trying things in a small binary and for a browser tab; a
//! thread that grows wants a native database. Which end of a declared store
//! a launch reaches is `beet_net`'s and `beet_infra`'s to say, in their
//! READMEs.
//!
//! A thread is a partition of that store ([`ThreadStore`]), named by its
//! thread id, its own isolated database. The root partition holds each
//! directory's index. No query or atomic batch spans two threads.
//!
//! | partition | table | key | holds |
//! | --- | --- | --- | --- |
//! | root | `thread_index` | `{dir}//{thread}` | the thread is listed in the directory |
//! | thread | `thread` | thread id | the [`Thread`] row |
//! | thread | `actor` | actor id | an [`Actor`] |
//! | thread | `post` | post id | a [`Post`] |
//! | thread | `response_meta` | post id | a [`ResponseMeta`] |
//!
//! A directory never holds an empty segment, so no param can spell the `//`,
//! and a directory's own threads are contiguous in the index, apart from a
//! nested directory's: its newest is one reverse scan of one row. Creating a
//! thread writes its actors, seed and thread row in one batch, then its index
//! row, so a listed thread is whole and a crash between the two leaves an
//! unlisted orphan. A write is a row, never a rewrite of the thread, so two
//! writers on one thread never erase each other.
//!
//! - **Keys.** A `ThreadId` and a `PostId` are typed `Tid`s, so an id carries
//!   its time and orders by it; a post's is minted after the newest post its
//!   window knows. An `ActorId` is an rkey, by default the actor's name in
//!   kebab-case, and two actors of one id in a scene are refused.
//!
//! # Posts
//!
//! A post's row is a small envelope every kind shares and one body per kind.
//! Its id carries its time and its partition is its thread, so it stores
//! neither:
//!
//! ```json
//! {"author":"ann",  "kind":"text",        "status":"completed", "body":{"text":"hi"}}
//! {"author":"coach","kind":"tool_output", "status":"completed", "subject":"3mw..", "body":{"call_id":"c1","text":".."}}
//! ```
//!
//! - **Status.** `in_progress` while it streams, then `completed`,
//!   `interrupted` where an interrupt closed it, or `deleted`.
//! - **Subject.** The post this one is about: the call an output answers, the
//!   post edited or deleted, the last post a compaction covers. Two indexes
//!   serve the reads, `by_subject` over `subject` and `by_kind` over `kind`
//!   then `author`.
//! - **Kind.** What the post is, from the closed list below. A post of any
//!   other kind is a log, a downstream's kind prefixing its crate, ie
//!   `beet_eval/decision`, its body that kind's fields ([`LogKind`]).
//!
//! | kind | body | a model reads | a view shows |
//! | --- | --- | --- | --- |
//! | `text` | `text` | its text, in its author's role | its text |
//! | `url` | `url`, `media_type`, `file_stem`? | an image as an image by url; on open responses any other as a file by url, on completions a video as `[Video: <url>]` and any other as its url in text | its url |
//! | `media` | `blob`, `file_stem`? | its bytes (Bytes, below): an image as an image; on open responses any other as a file, on completions a text file as its text and any other as a line naming it | its media type and size |
//! | `refusal` | `text` | its text, in its author's role | its text |
//! | `error` | `text` | its text, in its author's role | its text, as an error |
//! | `reasoning` | `text` | nothing, whoever wrote it | its text |
//! | `reasoning_summary` | `text` | nothing, whoever wrote it | its text |
//! | `tool_call` | `name`, `call_id`, `arguments` | the reading agent's own as a call; another's with its output as one message in the author's role | its arguments |
//! | `tool_output` | `call_id`, `text`, `files`?, each `blob` and `file_stem`?; `subject` its call | for the reading agent's own call, its output at once after the call, its files as content parts on open responses and on completions as the output's text then one user message holding them all; for another's, part of the call's message | its text, then each file's line |
//! | `session_opened` | `spawned_by`? | a join's note (Sessions) | `── Coach joined ──` |
//! | `edited` | `was`, the original body; `subject` the post | a note where the change was made, `<system>Ann edited a post from 2026-11-20T09:10Z.</system>`, the time the post's own | no row: the post shows its new body and that it was edited |
//! | `deleted` | none; `subject` the post | a note as an edit's | no row: the post shows that it was deleted, with no body |
//! | `compaction` | `text`, `from`, `previous`, `seed`; `subject` the last post it covers | its author's newest, unless stale, as a note opening its context (Contexts and compaction); no other | a divider saying how far it covers |
//! | any other log | that kind's fields | nothing | a divider naming its kind |
//!
//! A post's **role** is relative to the agent reading it, the same on every
//! protocol: a System actor's is a system message and a Developer's a
//! developer message, neither wrapped; the reading agent's own is native, an
//! assistant message and its calls and their outputs call items; anyone
//! else's is a user message wrapped in a `<post author=..>` naming its
//! author, so a group chat's people read apart. Another actor's call is never
//! sent as a call, since the reader did not make it. A note is a user message
//! whoever it is about. A blob a provider cannot carry is one line naming it
//! ([`PostFile::line`]), ie `[file: report.pdf, application/pdf, 12 KB]`.
//!
//! # Bytes
//!
//! Bytes are never in a row. A `media` post holds a `BlobRef` ([`PostFile`]),
//! its bytes at `blobs/<cid>` in the nearest `BlobStore` at or above the
//! thread, as a PDS holds them. They are written before the post, so a stored reference always
//! resolves, and addressed by content, so the same bytes twice are one blob.
//! Where that blob store is the repo store, the thread is as public as the
//! repo, which is intended. A thread with no store above it holds bytes in
//! memory on its window, bounded to what its contexts keep. A deleted or
//! edited post's blob is kept, as its row is.
//!
//! A request is built in two steps ([`RequestBlobs`](crate::prelude::RequestBlobs)): the records answer its
//! posts, then the bytes they reference are read, so neither mapper reads a
//! store. A missing blob is the text `[file unavailable]` to the model, never
//! a failed turn. [`KeepBlobs`]`{count}` on an agent carries the newest
//! `count` blobs in its context and lists each older one as a line naming its
//! post, media type, name and size; without it every blob is carried. Records
//! are never rewritten to shrink a request. A tool may answer with bytes,
//! kept as its output's `files`, so any route answering media is a tool an
//! agent can look through; an answer of several files is one
//! `multipart/mixed` body (RFC 2046, [`Multipart`](beet_net::prelude::Multipart)),
//! split into the output's text and each file. [`KeepBlobs`] counts files,
//! whatever post holds them, and a listing names a file by its post and its
//! index there, ie `[file: cat.png, image/png, 3 B] file 2 in post 3mw..`
//! ([`PostFile::listing`]). A listed blob a tool reads back is the newest,
//! carried again. Encoding bytes for a provider,
//! base64 included, is that provider's mapper's own, and a reference is never
//! sent as a url, since a provider could not fetch it.
//!
//! # Windows
//!
//! - **Window.** A thread entity, open onto one stored thread, which is the
//!   truth. It holds a contiguous run of that thread's records ending at its
//!   newest post ([`ThreadRecords`]) and reads older ones as needed
//!   ([`ThreadStore::read_older`]), writes what it posts to the store and
//!   receives what other windows post. How far back it opens is its
//!   [`Reach`]: a view's window opens on enough posts to fill it
//!   ([`MountThreadStore::load`]) and holds what its reader has scrolled to;
//!   a canonical window holds at least its agents' contexts, back through
//!   each agent's newest compaction where every one compacts, else the whole
//!   thread, and always the seed; every run reaches back to the last word. A post in progress stays
//!   in memory and is stored when it completes. A view keeps one small
//!   document per loaded post, written only when that post changes, and
//!   every reader of the records, a view, window sync and the flush, asks
//!   what changed since it last looked ([`ThreadRecords::changed_since`]),
//!   so its work follows the change and never the thread's length. A thread
//!   has any number of windows, on any surfaces, each with its own view and
//!   composer.
//! - **Window sync.** Within one runtime, a post pushed to one window's
//!   records, and any change to it, is applied to every other window of the
//!   same stored thread that loaded it at once ([`ThreadChanges`]), a post in
//!   progress included, so a reply streams in every window; a change to a
//!   post older than a window's run is dropped by it. A window joining a
//!   live one takes the posts the store may not hold yet and reads the rest
//!   from the store. Only a completed post is written to the store, once, by
//!   the window that authored it; a window never writes a post it received.
//! - **Behavior.** A scene holding an action on an actor, ie a `Sequence`
//!   over an agent's streamer and a person's [`UserInput`](crate::prelude::UserInput).
//! - **Canonical window.** The window holding a thread's behavior; a thread
//!   has at most one in a runtime ([`ThreadWindows`]), and every other window
//!   is a view and a composer with no behavior. Entering a thread with
//!   behavior while another window holds behavior for it is refused, naming
//!   both: two agents answering is a mistake in what was installed, never a
//!   race to resolve. A post from any window reaches the canonical window by
//!   window sync, so a person typing in a browser tab is answered, or
//!   interrupts, exactly as one typing beside it.
//!
//! A window lives as long as the scene holding it.
//! [`RunThread`](crate::prelude::RunThread) holds a
//! canonical window for the runtime's life, entered with the start request,
//! which is how a thread a person sits in is held.
//!
//! - **Thread route.** A thread run as a function
//!   ([`ThreadRoute`](crate::prelude::ThreadRoute)): built for one request, run to its outcome, answered and despawned, nobody typing.
//!   It refuses a template holding a [`UserInput`](crate::prelude::UserInput), and a request for a thread
//!   whose canonical window is live answers `409`.
//!
//! # Entering
//!
//! [`Thread::enter`] is the one place that knows whether a thread is created or
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
//! 5. load the newest stored posts the window opens on, take a live window's
//!    unstored ones, post a join for each actor present that no other window
//!    of the runtime holds ([`PresentActors`]), then post the request's
//!    message if the entry carries one.
//!
//! A seed source is resolved by the entry creating its thread and by nothing
//! else, so building a template reads no file and calls no route.
//!
//! A request's message belongs to the request:
//! `ThreadRoute{source:Document(".."), message:"review {document}", actor:"ann"}`, each
//! `{param}` filled from the request and posted after the joins, on every
//! request. Without `actor` its author is the first actor of kind User the
//! scene authors, and a thread holding none fails the request.
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
//! - **Note.** How a model reads a join, one convention for every provider
//!   ([`ThreadRecords::note`]):
//!   `<system>Coach joined at 2026-11-22T09:10Z, 2 days 3 hours after the last post.</system>`.
//!   The gap is measured from the last post that is not a log, and omitted
//!   when there is none or it is under a minute. Consecutive notes, joins,
//!   edits and deletions alike, are one message, a line each inside one
//!   `<system>`. A note is sent in the user
//!   role whoever joined, the reading agent's own join included
//!   ([`ModelPost::Note`]): it is something that happened, never something
//!   the agent said. An edit, a deletion and a compaction are read as notes
//!   too (Posts); no other log reaches a model.
//!
//! # Posting and turns
//!
//! The records are the bus. Any actor present may post at any time, and
//! posting never requires holding a turn: a composer appends its post the
//! moment it is submitted.
//!
//! - **Last word.** The newest post that is neither a log nor a
//!   `tool_output` closing an interrupted call, which nobody said
//!   ([`ThreadRecords::last_word`]).
//! - **Turn.** An action. A thread's behavior decides when each actor's
//!   action runs, and that is the whole of turn taking; a thread with no
//!   behavior takes posts from whoever is present. A person's turn
//!   ([`UserInput`](crate::prelude::UserInput)) is a wait on the records, passing once the last word is
//!   its actor's.
//! - **Interrupt.** A running turn cut short ([`InterruptTurn`](crate::prelude::InterruptTurn)), beet's own
//!   and the same for every provider, since no request API takes input
//!   during a response: the model's stream is dropped, the post in progress
//!   closed as interrupted, the turn's tool calls cancelled, each given an
//!   output saying so, and the turn passes. [`InterruptOnPost`](crate::prelude::InterruptOnPost) on an agent is
//!   that agent's manners: any post by another actor that is not a log
//!   interrupts it, so a join interrupts nobody.
//!
//! A model reads its context in model order ([`ThreadRecords::model_posts`]):
//! a `tool_call` is followed at once by the `tool_output` whose subject it is,
//! and a post landing between them is read after the output, so a post made
//! mid tool call never breaks a provider's pairing of the two.
//!
//! A thread has one of three **shapes**, each the shape of its behavior and
//! its actors, never a mode of the thread, and each independent of whether an
//! actor speaks or types:
//!
//! - **Turn based.** A `Sequence` of actors, one acting at a time. A post made
//!   while an agent works stops nothing and is not lost: every model call
//!   reads the agent's whole context, so the agent reads it at its next
//!   call, ie between two tool calls of the same turn.
//! - **Semi duplex.** Turn based with interrupts: its agents carry
//!   `InterruptOnPost`, so a post, or the composer's stop control, cuts the
//!   running turn and the `Sequence` moves on, answering the person now. It
//!   is the same `Sequence`, since a person's turn already passes when the
//!   last word is theirs.
//! - **Full duplex.** Actors under a `Parallel` act at once and may speak over
//!   each other. Nothing interrupts anybody on another's behalf: whether an
//!   actor yields when another speaks is its own `InterruptOnPost`.
//!
//! # Editing and deleting
//!
//! Only its author may edit or delete a post, and only a completed `text` or
//! `media` post of the thread itself, never one a branch inherits
//! ([`Post::check_change`]). An edit ([`EditPost`]) replaces the row's body
//! and appends an `edited` log holding the original ([`PostEdited`]). A
//! deletion ([`DeletePost`]) is a tombstone: the row's status becomes
//! `deleted` and its body stays, with a `deleted` log saying when
//! ([`PostDeleted`]). Each is one atomic batch, the row replaced under the
//! cid its window read it at, so a conflicting change is refused to whoever
//! made it, never retried over another's. Window sync carries the change to
//! every window of the runtime that loaded the post, and only the window
//! making it writes it. Nothing is scrubbed, and a post changed twice has
//! two logs, so its history reads by `by_subject`.
//!
//! # Everything and shown
//!
//! The store and a window's records hold everything ([`ThreadStore::posts`],
//! [`ThreadRecords::posts`]). **Shown** is a thread without what was edited
//! away or deleted: [`ShownPosts`], derived from the records in memory and
//! stored nowhere, the same posts with a deleted post's body, an edit's
//! original and a compaction that may no longer be read each emptied. A
//! deleted post stays in it, so a view can say it was there, and
//! [`AgentPost`] reads it as deleted and nothing else. It is the whole list
//! as one immutable type, made in one place ([`ThreadRecords::shown`]), so a
//! list in hand is known clean and no reader judges a post at a time; a post
//! nobody touched is the records' own, shared rather than copied.
//!
//! | reader | reads |
//! | --- | --- |
//! | a view, a composer, a browser tab, a `ThreadRoute` answer | shown |
//! | an agent in the conversation, its context and its tools | shown |
//! | a compaction thread, its message and its `<ReadPosts/>` | everything |
//! | `ServeTables`, a program asking for `posts` by name | everything |
//!
//! Who reads what is decided by where a reader sits, never by a flag an
//! author could set.
//!
//! # Contexts and compaction
//!
//! - **Context.** What one model call reads ([`ThreadRecords::model_posts`]):
//!   the seed unless a compaction holds it, the reading agent's newest
//!   compaction as a note ([`ThreadRecords::compaction_note`]), then every
//!   shown post after that compaction's subject, with the notes among them.
//!   With no compaction it is the whole shown thread. Its size is the last
//!   response's recorded input, plus the estimate of what followed what that
//!   response read ([`ThreadRecords::context_tokens`]), the one estimate a
//!   quarter of a post's row length and a thousand a file
//!   ([`ThreadRecords::estimate`]).
//! - **Chaining.** A request continuing a provider's stored response sends
//!   only what follows the newest post that response read (its
//!   [`ResponseMeta`]'s `read`), never chaining to one whose `read` covers a
//!   post changed since ([`ThreadRecords::stored_response`]), but to the
//!   newest that does not, else sending everything; a compaction starts a
//!   new chain, so no response older than the agent's newest is chained to.
//! - **Compaction.** A stored summary of everything up to one post, a
//!   `compaction` post authored by the agent whose context it is
//!   ([`Compaction`]: its text, the first post it was given verbatim, the
//!   compaction it folds in and whether the seed is inside). It is opt in,
//!   on the agent ([`Compact`](crate::prelude::Compact)`{template, at, keep, step, method, seed, backfill}`),
//!   and done by another: a thread of its own, built from the named template
//!   for one compaction, run as a function and despawned, its model named
//!   there and never assumed to be the conversation's. It is told the newest
//!   summary, what changed under it and what to fold in, may read the
//!   thread's posts through its one tool
//!   ([`<ReadPosts/>`](crate::prelude::ReadPosts)), is seeded with the rule
//!   ([`<CompactInstructions/>`](crate::prelude::CompactInstructions)), and
//!   answers the new summary. It runs inside the agent's turn, before a model
//!   call, so an interrupt drops it and leaves the older one in force. Every
//!   post is summarised or read verbatim, with no gap: each compaction is
//!   given the posts after the last one's subject, and the boundary never
//!   falls between a call and its output or on a post in progress.
//! - **Stale.** A compaction covering a post edited or deleted after its
//!   message was built ([`ThreadRecords::is_stale`]). Only an agent's newest
//!   compaction is read ([`ThreadRecords::compaction`]), and never a stale
//!   one: it is refreshed before the agent's next model call, one call however
//!   many changed, and a refresh that fails ends the turn on its error.
//!
//! What was edited away or deleted is sensitive: a compaction thread reads
//! it, to know what to take out, and a summary holds none of it. People see
//! the original posts wherever nobody edited or deleted them, and a view
//! shows a compaction as a divider saying how far it covers, never its text.
//!
//! # Tools
//!
//! - **Tool.** A route of an agent's own `<Router>`, its identity its route
//!   path, so a call of anything else is that router's not-found. A route of
//!   the surrounding url space becomes a tool through
//!   [`<RouteTool/>`](crate::prelude::RouteTool).
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
//!   a post, then going its own way, reading its ancestors' partitions rather
//!   than copying them. A branch lives in its parent's store, so
//!   `--branch-of` naming a post the mount's store does not list is refused,
//!   as is `--branch-of` beside `--thread`. It opens with its parent's posts,
//!   so creating a branch reads no seed, and its seed is its lineage root's.
//!
//! # Limits
//!
//! Recorded here and built by nothing:
//!
//! - every surface of a template is the same User actor, since people have no
//!   identity yet;
//! - no route lists the threads of a store;
//! - window sync is one runtime's, so windows in two runtimes see each other's
//!   posts, and an edit, only on entering, and a backend's own change feed is
//!   unused;
//! - an actor's leaving is not recorded: presence is a status nothing keeps;
//! - nothing erases for good, a deletion being a tombstone, and what another
//!   post repeats of a deleted one stays, an agent's own reply included,
//!   until its author removes it;
//! - an agent holding a tool over the blob store a `JsonTableStore` sits in,
//!   or one reaching `ServeTables`, reads the rows themselves, so such a
//!   thread is declared over a directory of its own;
//! - a summary leaves out what was deleted because its model is told to,
//!   which nothing can prove;
//! - a `JsonTableStore` rewrites its partition on each write, and a browser's
//!   local storage holds about 5 MB in all.
mod actor;
mod create_actor;
pub use create_actor::*;
mod create_post;
mod record_id;
pub use record_id::*;
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
mod thread_store;
#[cfg(feature = "action")]
pub use seed_source::*;
pub use thread_store::Reach;
pub use thread_store::SyncedRecords;
#[cfg(feature = "action")]
pub(crate) use thread_store::ThreadFlush;
pub use thread_store::ThreadStore;
pub(crate) use thread_store::sync_records_to_store;
mod agent_post;
pub use agent_post::*;
mod post;
mod post_file;
pub use post_file::*;
mod post_log;
mod post_ref;
pub use post_ref::*;
mod session;
mod window;
pub use actor::*;
pub use post::*;
pub use post_log::*;
pub use session::*;
pub use thread::*;
pub use thread_query::*;
pub use thread_records::*;
pub use window::*;
mod response_meta;
pub use response_meta::*;
mod compaction;
pub use compaction::Compaction;
mod shown_posts;
pub use shown_posts::ShownPosts;
mod edit_post;
pub use edit_post::*;
mod revision;
pub use revision::PostRevision;
pub use revision::Revision;
pub(crate) use revision::RevisionLog;
mod keep_blobs;
pub use keep_blobs::KeepBlobs;
pub(crate) use keep_blobs::release_bytes;
