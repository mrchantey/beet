//! A thread's records, in the store and in memory.
//!
//! **Entities are for what runs, records are for what is remembered.** A
//! [`Thread`], [`Actor`] or [`Post`] is a serializable record named by its id
//! ([`ThreadId`], [`ActorId`], [`PostId`]), kept in a [`ThreadStore`]; an
//! entity is a live process running or showing a record, and one thread is
//! many entities over its life, each session ([`Thread::enter`]) a fresh one
//! adopting the stored id. Posts are never entities: the reduction
//! ([`ThreadRecords::reduce`]) folds actors and seed posts into the
//! [`ThreadRecords`] on the thread entity, and an actor survives as an
//! entity only as behavior, an [`ActorRef`] beside its action. Anything
//! persisted refers to another thread or post by id, never by `Entity`, and a
//! live view joins records with live entities by id.
//!
//! Two relations between threads live on the [`Thread`] record, each naming a
//! post by [`PostRef`], independent of each other:
//!
//! - **spawned** ([`Thread::spawned_by`]): started by an action in another
//!   thread, ie a loop decision or a delegate call. Causal only, no context
//!   comes with it.
//! - **branched** ([`Thread::branch_of`]): continues the history of another
//!   thread up to a post, then goes its own way. A post may have many
//!   branches and a thread at most one parent, so branches form a tree, and
//!   a branch's history references its ancestors' posts rather than copying
//!   them ([`ThreadStoreProvider::history`]).
//!
//! A thread continues across sessions by the metadata its route declares
//! ([`ThreadEntry`]), never its seed. A seed is the thread's initial posts,
//! kept by a fresh thread; a continued thread enters with its stored seed,
//! so a changed seed reaches a new thread (`--new`), and with its whole
//! history. Each session is
//! bracketed by its [`SessionOpened`] and [`SessionClosed`] posts.
mod actor;
mod create_actor;
pub use create_actor::*;
mod create_post;
mod document;
mod load_thread;
pub(crate) mod request_post;
pub use create_post::*;
pub use load_thread::*;
pub use request_post::*;
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
