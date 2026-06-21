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
			// agent-loop control flow + the standard store toolset, as markup
			.register_type::<RepeatWhileFunctionCallOutput>()
			.register_type::<RunThread>()
			.register_template::<StoreToolset>()
			.register_template::<MountFsStore>()
			// markup kick: call the thread on load, exit when it completes
			.add_systems(Update, run_thread_on_load);

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
			.register_type::<O11sStreamer>()
			.register_type::<CompletionsStreamer>()
			.register_type::<Provider>()
			.register_type::<ModelApi>()
			.register_type::<ModelSize>()
			// ── Reasoning sub-types ───────────────────────────────────────
			.register_type::<ReasoningEffort>()
			.register_type::<ReasoningSummary>()
			.register_type::<ReasoningParam>()
			// ── Tool definition types ─────────────────────────────────────
			.register_type::<ToolDefinition>()
			.register_type::<FunctionToolDefinition>()
			.register_type::<ProviderToolDefinition>()
			.register_type::<ToolChoice>()
			// ── Markup templates ──────────────────────────────────────────
			.register_template::<CreatePost>()
			.register_template::<ModelStreamer>()
			.register_template::<CreateActor>()
			.add_observer(insert_tool_definition)
			// _
			;

		app.add_systems(First, ThreadWindow::reduce)
			.add_systems(PostUpdate, thread_store::sync_window_to_store);
	}
}
