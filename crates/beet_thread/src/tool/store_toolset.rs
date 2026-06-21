use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// `<StoreToolset/>` nested under an agent in a `.bsx` equips the five routed
	/// blob tools, each deriving a [`ToolDefinition`] the agent sends to the model.
	#[beet_core::test]
	fn store_toolset_equips_five_tools() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<ThreadPlugin>();
		let source = r#"
<div {Thread}>
	<CreateActor name="Coder" kind="Agent" {ModelStreamer{provider:Ollama}}>
		<StoreToolset/>
	</CreateActor>
</div>
"#;
		BsxTemplate::parse_entry(app.world(), source)
			.unwrap()
			.spawn(app.world_mut())
			.unwrap();
		ThreadWindow::reduce_now(app.world_mut());
		app.world_mut().flush();

		app.world_mut()
			.query::<&ToolDefinition>()
			.iter(app.world())
			.count()
			.xpect_eq(5);
	}
}
