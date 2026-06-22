//! The standard blob-store agent toolset and a markup store mount.
//!
//! These compose [`exchange_route`] with `beet_net`'s blob-store actions, so they
//! live with the other `extra` router pieces (eg [`BlobStoreRoute`]) rather than
//! in a downstream crate. An agent crate re-exports them for its scenes.

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

/// Mount a filesystem-backed [`BlobStore`] from markup, so a [`StoreToolset`]
/// nested under the same root resolves it without Rust glue:
/// `<div {Thread} {Sequence} {MountFsStore{path:"target/examples/agent"}}>`.
///
/// The `path` is workspace-relative; [`FsStore`]'s own `path` is an `AbsPathBuf`
/// (not attribute-coercible), so this thin template adapts a coercible string.
#[template]
pub fn MountFsStore(#[prop(into)] path: String) -> impl Bundle {
	FsStore::new(WsPathBuf::new(path))
}

// `<StoreToolset/>` equipping the five routed blob tools (and the `ToolDefinition`
// an agent derives from each) is covered downstream where the tool-definition
// observer lives: `tests/thread_scenes.rs` (`coding_agent`/`self_evolving` reduce
// to a `tool_count` of 5).
