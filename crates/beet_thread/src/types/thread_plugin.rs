use crate::o11s::ReasoningEffort;
use crate::o11s::ReasoningSummary;
use crate::o11s::request::ReasoningParam;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

#[cfg(feature = "action")]
use beet_action::prelude::*;

#[derive(Default)]
pub struct ThreadPlugin {}

impl Plugin for ThreadPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>()
			.init_plugin::<RouterPlugin>()
			.init_plugin::<NetPlugin>();

		#[cfg(feature = "action")]
		app.init_plugin::<ActionPlugin>()
			// agent-loop control flow, as markup. `StoreToolset` is registered
			// upstream by `RouterPlugin` (init above); the store is a plain `FsStore`.
			.register_type::<RepeatWhileFunctionCallOutput>()
			// markup verb: the thread's `Request -> Outcome` entry point, plus the
			// `CallOnStart` that calls it on the entry's start. Owns no action slot;
			// `ServerPlugin` (via `RouterPlugin`) owns the start fan-out.
			.register_type::<RunThread>()
			// markup persistence: declare a thread-record store from `.bsx`
			.register_type::<MountThreadStore>()
			// markup window bounding: stub older images so an endless loop's
			// request stays bounded without dropping a post
			.register_type::<StubOldImages>()
			// the OpenResponses streamer is an action
			.register_type::<O11sStreamer>()
			.add_observer(insert_tool_definition);

		// the async-openai-backed completions streamer, plus the model selection
		// driven by the provider constructors beside it
		#[cfg(feature = "agent")]
		app.register_type::<CompletionsStreamer>()
			.register_type::<Provider>()
			.register_type::<ModelApi>()
			.register_type::<ModelSize>()
			.register_template::<ModelStreamer>();

		app
			// ── Uuid7 instantiations ─────────────────────────────────────
			.register_type::<Uuid7<Thread>>()
			.register_type::<Uuid7<Actor>>()
			.register_type::<Uuid7<Post>>()
			// ── Table types ───────────────────────────────────────────────
			.register_type::<Thread>()
			.register_type::<Actor>()
			.register_type::<ActorKind>()
			.register_type::<Post>()
			.register_type::<PostIntent>()
			.register_type::<Timestamp>()
			.register_type::<ResponseMeta>()
			.register_type::<ActorRef>()
			.register_type::<ThreadConfig>()
			// ── Streaming types ───────────────────────────────────────────
			.register_type::<EnvVar>()
			.register_type::<ModelDef>()
			// ── Reasoning sub-types ───────────────────────────────────────
			.register_type::<ReasoningEffort>()
			.register_type::<ReasoningSummary>()
			.register_type::<ReasoningParam>()
			// ── Tool definition types ─────────────────────────────────────
			.register_type::<ToolDefinition>()
			.register_type::<FunctionToolDefinition>()
			.register_type::<ProviderToolDefinition>()
			.register_type::<ToolChoice>()
			.register_type::<StringEnumOptions>()
			// ── Markup templates ──────────────────────────────────────────
			.register_template::<CreatePost>()
			.register_template::<CreateActor>()
			.add_observer(reapply_string_enum_options)
			// _
			;

		app.add_systems(First, ThreadWindow::reduce)
			.add_systems(Update, sync_string_enum_options)
			.add_systems(PostUpdate, thread_store::sync_window_to_store);
	}
}
