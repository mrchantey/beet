//! The standard blob-store agent toolset.
//!
//! This composes [`exchange_route`] with `beet_net`'s blob-store actions, so it
//! lives with the other `extra` router pieces (eg [`ServeBlobs`]) rather than
//! in a downstream crate. An agent crate re-exports it for its scenes.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Equip an agent with the standard blob-store toolset: list, read, write,
/// edit, and remove against the nearest ancestor [`BlobStore`]. Each entry is a
/// routed [`exchange_route`], so the agent both sees the tool's schema and can
/// dispatch the call.
///
/// A `#[template]`, so it nests under an agent in markup, ie
/// `<CreateActor name="Coder" kind="Agent" {ModelStreamer{provider:OpenAi}}><StoreToolset/></CreateActor>`,
/// with a [`BlobStore`] mounted on an ancestor (eg the thread's behavior root).
#[template]
pub fn StoreToolset() -> impl Bundle {
	children![
		exchange_route("list-blobs", ListBlobs),
		exchange_route("read-blob", ReadBlob),
		exchange_route("write-blob", WriteBlob),
		exchange_route("edit-text", EditText),
		exchange_route("remove-blob", RemoveBlob),
	]
}

// `<StoreToolset/>` equipping the five routed blob tools (and the `ToolDefinition`
// an agent derives from each) is covered downstream where the tool-definition
// observer lives: `tests/thread_scenes.rs` (`coding_agent`/`self_evolving` reduce
// to a `tool_count` of 5).
