use crate::prelude::*;
use beet_core::prelude::*;

/// Author an [`Actor`] in markup, ie `<CreateActor name="Agent" kind="Agent"/>`.
///
/// A plain [`Actor`] cannot be a BSX tag because its [`ActorId`] (a [`Uuid7`])
/// is not attribute-coercible; this template wraps it. Set `id` to pin a stable
/// identity (required for persisted threads, where the seed hash and `ActorRef`
/// bindings depend on it); omit it for an ephemeral, freshly-minted id.
///
/// Spread behavior and nest seeds/tools as children, ie
/// `<CreateActor name="Agent" kind="Agent" {ModelStreamer{provider:OpenAi}}>
///   <CreatePost text="hello"/>
/// </CreateActor>`.
#[template]
pub fn CreateActor(
	#[prop(into)] name: String,
	kind: ActorKind,
	id: Option<u64>,
) -> impl Bundle {
	let actor = match id {
		Some(id) => {
			Actor::new_with_id(ActorId::from_u128(id as u128), name, kind)
		}
		None => Actor::new(name, kind),
	};
	// forward seed posts / tools as children of the actor entity via `<Slot/>`
	rsx! { <span {actor}><Slot/></span> }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::PathPartial;
	use beet_router::prelude::*;

	/// Probe input for the markup-tool experiment.
	#[derive(Reflect, serde::Serialize, serde::Deserialize)]
	struct ProbeInput {
		/// a field
		value: String,
	}

	/// A routed tool, the shape of `tool_call`'s `AgentChoiceAction`.
	#[action(pure, route = "probe-tool")]
	#[derive(Component, Reflect)]
	#[reflect(Component)]
	fn ProbeTool(cx: ActionContext<ProbeInput>) -> String { cx.value.clone() }

	/// A `kind="..."` attribute resolves to the named [`ActorKind`] variant rather
	/// than silently defaulting to `Agent`: a string attribute coercing to a unit
	/// enum variant by name (`kind="User"` -> `ActorKind::User`).
	#[beet_core::test]
	fn kind_attribute_resolves_to_variant() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).init_plugin::<ThreadPlugin>();
		let source = r#"
<div {Thread}>
	<CreateActor name="Sys" kind="System"/>
	<CreateActor name="Bot" kind="Agent"/>
	<CreateActor name="Person" kind="User"/>
</div>
"#;
		BsxTemplate::parse_entry(app.world(), source)
			.unwrap()
			.spawn(app.world_mut())
			.unwrap();
		ThreadWindow::reduce_now(app.world_mut());

		let thread = app
			.world_mut()
			.query_filtered::<Entity, With<Thread>>()
			.iter(app.world())
			.next()
			.unwrap();
		let kinds = app
			.world()
			.get::<ThreadWindow>(thread)
			.unwrap()
			.actors()
			.values()
			.map(|actor| actor.kind())
			.collect::<Vec<_>>();
		kinds.contains(&ActorKind::System).xpect_true();
		kinds.contains(&ActorKind::User).xpect_true();
		kinds.contains(&ActorKind::Agent).xpect_true();
	}

	/// A routed `#[action]` referenced by tag in a runtime `.bsx` equips the same
	/// tool an `rsx!` `children![Tool]` would: reflect-inserting the component
	/// fires its `#[require]` chain (`Action`/`ExchangeOverload`/`PathPartial`) and
	/// the tool-definition pipeline derives the [`ToolDefinition`] the agent sends
	/// to the model. This is the contract the markup examples lean on.
	#[beet_core::test]
	fn bsx_tool_tag_fires_requires() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<ThreadPlugin>();
		app.register_type::<ProbeTool>();

		let source = r#"
<div {Thread}>
	<CreateActor name="Agent" kind="Agent" {ModelStreamer{provider:Ollama}}>
		<ProbeTool/>
	</CreateActor>
</div>
"#;
		let template = BsxTemplate::parse_entry(app.world(), source).unwrap();
		template.spawn(app.world_mut()).unwrap();
		app.world_mut().flush();

		// the tool entity gained its routed requires + the derived tool definition
		let tool = app
			.world_mut()
			.query_filtered::<Entity, With<ProbeTool>>()
			.single(app.world())
			.unwrap();
		app.world().get::<ExchangeOverload>(tool).xpect_some();
		app.world().get::<PathPartial>(tool).xpect_some();
		app.world().get::<ToolDefinition>(tool).xpect_some();
	}

	/// A long, multi-line seed prompt survives a `.bsx` `text=` attribute
	/// verbatim, using single quotes so embedded double quotes do not terminate
	/// it (the tool_call / coding_agent prompt shape).
	#[beet_core::test]
	fn bsx_seed_preserves_multiline_prompt() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<ThreadPlugin>();
		let source = "
<div {Thread}>
	<CreateActor name=\"System\" kind=\"System\">
		<CreatePost text='line one
respond: \"I open the door..\"
line three'/>
	</CreateActor>
</div>
";
		let thread = BsxTemplate::parse_entry(app.world(), source)
			.unwrap()
			.spawn(app.world_mut())
			.unwrap();
		ThreadWindow::reduce_now(app.world_mut());

		let window = app.world().get::<ThreadWindow>(thread).unwrap();
		window.posts()[0].to_string().xpect_eq(
			"line one\nrespond: \"I open the door..\"\nline three".to_string(),
		);
	}

	/// A `.bsx` author scene reduces into a `ThreadWindow` + behavior scene just
	/// like a Rust one: `<CreateActor>` actors, `<CreatePost>` seeds, a spread
	/// `{ModelStreamer}` behavior.
	#[beet_core::test]
	fn bsx_author_scene_reduces() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<ThreadPlugin>();

		// the root carries `Thread` via a spread on a lowercase element (which
		// nests children natively); actors are `<CreateActor>` tags that forward
		// their seed/tool children through a `<Slot/>`.
		let source = r#"
<div {Thread}>
	<CreateActor name="System" kind="System">
		<CreatePost text="be helpful"/>
	</CreateActor>
	<CreateActor name="Agent" kind="Agent" {ModelStreamer{provider:Ollama}}/>
</div>
"#;
		let template = BsxTemplate::parse_entry(app.world(), source).unwrap();
		let thread = template.spawn(app.world_mut()).unwrap();
		ThreadWindow::reduce_now(app.world_mut());

		// the window holds both actors and the seed post
		let window = app.world().get::<ThreadWindow>(thread).unwrap();
		window.actors().len().xpect_eq(2);
		window.posts().len().xpect_eq(1);
		window.posts()[0]
			.to_string()
			.xpect_eq("be helpful".to_string());

		// the agent reduced to a single ActorRef carrying its streamer behavior;
		// the seed-only system actor was despawned
		app.world_mut()
			.query_filtered::<Entity, (With<ActorRef>, With<O11sStreamer>)>()
			.iter(app.world())
			.count()
			.xpect_eq(1);
		app.world_mut()
			.query::<&Actor>()
			.iter(app.world())
			.count()
			.xpect_eq(0);
	}
}
