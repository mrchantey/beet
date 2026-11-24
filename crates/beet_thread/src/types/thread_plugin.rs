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
		app.install_type::<(AsyncPlugin, RouterPlugin, NetPlugin)>();
		// a `PostRef` and each id read from a flag, a param or markup
		PostRef::register_literal();
		ThreadId::register_literal();
		ActorId::register_literal();
		PostId::register_literal();

		#[cfg(feature = "action")]
		app.install_type::<ActionPlugin>()
			// agent-loop control flow, as markup. `StoreToolset` is registered
			// upstream by `RouterPlugin` (init above).
			.register_type::<RepeatWhileFunctionCallOutput>()
			// markup verb: the thread's `Request -> Outcome` entry point, plus the
			// `CallOnStart` that calls it on the entry's start sweep. Owns no
			// action slot.
			.register_type::<RunThread>()
			// a route building a thread per request
			.register_type::<ThreadRoute>()
			// markup persistence: declare a thread-record store from `.bsx`
			.register_type::<MountThreadStore>()
			.register_type::<ThreadEntryParams>()
			// markup image bounding: stub older images so an endless loop's
			// request stays bounded without dropping a post
			.register_type::<StubOldImages>()
			// a host verb as an agent's tool, `<HostTool path=".."/>`
			.register_type::<HostRoute>()
			.install_type::<HostTool>()
			// a seed post read from a route's answer
			.install_type::<RoutePost>()
			// the OpenResponses streamer is an action
			.register_type::<O11sStreamer>()
			// turns: a person's waits on the records, an agent's may be cut
			// short
			.register_type::<UserInput>()
			.register_type::<InterruptOnPost>()
			.add_observer(interrupt_turn)
			.add_systems(
				PostUpdate,
				(interrupt_on_post, pass_person_turns).after(sync_windows),
			)
			.add_observer(insert_tool_definition);

		// the async-openai-backed completions streamer, plus the model selection
		// driven by the provider constructors beside it
		#[cfg(feature = "agent")]
		app.register_type::<CompletionsStreamer>()
			.register_type::<Provider>()
			.register_type::<ModelApi>()
			.register_type::<ModelSize>()
			// the scripted mock, so a scene runs its agents with no network
			.register_type::<MockPostStreamer>()
			.register_type::<MockTurn>()
			.install_type::<ModelStreamer>();

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
			.register_type::<PostRef>()
			.register_type::<Usage>()
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
			.install_type::<(CreatePost, CreateActor)>()
			.add_observer(reapply_string_enum_options)
			// _
			;

		app.add_systems(First, ThreadRecords::reduce)
			.add_systems(Update, sync_string_enum_options)
			.add_systems(
				PostUpdate,
				(sync_windows, sync_records_to_store).chain(),
			);
	}
}
