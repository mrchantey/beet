//! The genuine actions and components behind `examples/action/*.bsx`.
//!
//! Each `.bsx` drives a behaviour tree from markup through the one `beet`
//! binary, with the tree shape (names, stats, control flow) authored in BSX and
//! only the real new actions/components living here. The control-flow nodes
//! (`Sequence`, `Repeat`, `Fallback`, `HighestScore`, ...) and the load verb
//! (`RunOnLoad`) come from `beet_action`/`beet_net`.
//!
//! ```bsx
//! <Sequence {RunOnLoad}>
//!   <Log::Message("running child1")/>
//!   <Log::Message("running child2")/>
//! </Sequence>
//! ```
use beet_action::prelude::*;
use beet_core::prelude::*;
use core::time::Duration;

// ── hello_world ──────────────────────────────────────────────

/// `<Greet{name:".."}/>` - on load, greets the given name and passes. The
/// config is a component field, read by [`GreetAction`].
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(GreetAction)]
pub struct Greet {
	/// The name to greet.
	pub name: String,
}

/// Reads the caller's [`Greet`], logs `Hello, {name}!`, then passes.
#[action(default)]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
async fn GreetAction(cx: ActionContext) -> Result<Outcome> {
	let Greet { name } = cx.caller.get_cloned::<Greet>().await?;
	info!("Hello, {name}!");
	Outcome::PASS.xok()
}

// ── simple_action ────────────────────────────────────────────

/// Greets using the caller entity's [`Name`], then passes. Reads a component off
/// the caller via the async world handle on [`ActionContext`].
#[action]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
async fn SayHello(cx: ActionContext) -> Result<Outcome> {
	let name = cx
		.caller
		.get(|name: &Name| name.to_string())
		.await
		.unwrap_or_else(|_| "anonymous".to_string());
	info!("hello from {name}");
	Outcome::PASS.xok()
}

// ── long_running ─────────────────────────────────────────────

/// Patrols for a few steps, sleeping between each, then passes. A long-running
/// action is just a future that takes a while to resolve.
#[action]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
async fn Patrol(cx: ActionContext) -> Result<Outcome> {
	let _ = cx;
	for step in 1..=5 {
		time_ext::sleep(Duration::from_millis(200)).await;
		info!("patrolling, step {step}");
	}
	Outcome::PASS.xok()
}

// ── utility_ai / malenia ─────────────────────────────────────

/// Current health of a combatant.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
struct Health(f32);

/// How many healing potions a combatant is holding.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
struct HealingPotions(usize);

/// Heals the agent if its health is low and it has potions, else fails.
#[action]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
async fn TryHealSelf(cx: ActionContext) -> Result<Outcome> {
	let world = cx.world();
	let agent = AgentQuery::entity_async(&world, cx.caller.id()).await;
	world
		.entity(agent)
		.with(|mut agent| -> Outcome {
			let potions = agent
				.get::<HealingPotions>()
				.map(|potions| potions.0)
				.unwrap_or(0);
			let health = agent
				.get::<Health>()
				.map(|health| health.0)
				.unwrap_or(f32::MAX);
			if potions == 0 || health >= 50.0 {
				return Outcome::FAIL;
			}
			if let Some(mut health) = agent.get_mut::<Health>() {
				health.0 += 30.0;
			}
			if let Some(mut potions) = agent.get_mut::<HealingPotions>() {
				potions.0 -= 1;
			}
			let remaining = agent
				.get::<Health>()
				.map(|health| health.0)
				.unwrap_or_default();
			info!("Malenia heals herself, health now {remaining}");
			Outcome::PASS
		})
		.await?
		.xok()
}

/// Marks the player entity, so an [`AttackPlayer`] resolves its target by query
/// rather than threading an [`Entity`] through markup.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
struct AttackTarget;

/// An attack: deals damage to the [`AttackTarget`] and recoil to the agent.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(AttackPlayerAction)]
struct AttackPlayer {
	max_damage: f32,
	max_recoil: f32,
}

/// Rolls damage and recoil, applies them, and reports the outcome of the round.
#[action(default)]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
async fn AttackPlayerAction(cx: ActionContext) -> Result<Outcome> {
	let attack = cx.caller.get_cloned::<AttackPlayer>().await?;
	let name = cx
		.caller
		.get(|name: &Name| name.to_string())
		.await
		.unwrap_or_else(|_| "an attack".to_string());
	let world = cx.world();
	let malenia = AgentQuery::entity_async(&world, cx.caller.id()).await;

	world
		.with(move |world: &mut World| -> Result<Outcome> {
			let player = world
				.with_state::<Query<Entity, With<AttackTarget>>, _>(|query| {
					query.iter().next()
				})
				.ok_or_else(|| bevyhow!("no AttackTarget entity"))?;
			let damage: f32 = world
				.resource_mut::<RandomSource>()
				.random_range(0.0..attack.max_damage)
				.round();
			let recoil: f32 = world
				.resource_mut::<RandomSource>()
				.random_range(0.0..attack.max_recoil)
				.round();
			info!("Malenia attacks with {name}");

			let player_hp = {
				let mut player_health = world
					.get_mut::<Health>(player)
					.ok_or_else(|| bevyhow!("player has no Health"))?;
				player_health.0 -= damage;
				player_health.0
			};
			info!("Player takes {damage} damage, health now {player_hp}");

			let malenia_hp = {
				let mut malenia_health = world
					.get_mut::<Health>(malenia)
					.ok_or_else(|| bevyhow!("Malenia has no Health"))?;
				malenia_health.0 -= recoil;
				malenia_health.0
			};
			info!("Malenia takes {recoil} recoil, health now {malenia_hp}");

			if player_hp <= 0.0 {
				info!("You lose - Malenia, Blade of Miquella, stands");
				return Outcome::FAIL.xok();
			}
			if malenia_hp <= 0.0 {
				info!("You win - 'Your strength, extraordinary...'");
				return Outcome::FAIL.xok();
			}
			Outcome::PASS.xok()
		})
		.await
}

/// A [`ScoreProvider`] returning a fresh random score on each evaluation.
fn random_score() -> ScoreProvider<()> {
	ScoreProvider(Action::<(), Score>::new_system(
		|_: In<ActionContext>,
		 mut rng: ResMut<RandomSource>|
		 -> Result<Score> { Score(rng.random::<f32>()).xok() },
	))
}

/// `{RandomScore}` - installs a [`ScoreProvider`] that returns a fresh random
/// score on each evaluation, so a [`HighestScore`] child without a fixed
/// [`Score`] still scores.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ScoreProvider<()> = random_score())]
struct RandomScore;

// ── state_machine ────────────────────────────────────────────

/// The next entity a [`StateAction`] jumps to, set in markup with an entity
/// reference: `{GoTo($next)}`. Held apart from the action so a terminal state
/// simply omits it. The target is required: an authored `<GoTo/>` with no entity
/// is rejected by the BSX loader.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct GoTo(#[reflect(@RequiredField)] pub Entity);

impl Default for GoTo {
	fn default() -> Self { Self(Entity::PLACEHOLDER) }
}

/// A traced state-machine node: logs entry, jumps to its [`GoTo`] target or
/// returns [`Outcome::PASS`] when terminal, then logs the result. The markup
/// counterpart of `trace_action.wrap(RunNext..)`.
#[action]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
async fn StateAction(cx: ActionContext) -> Result<Outcome> {
	let name = cx
		.caller
		.get(|name: &Name| name.to_string())
		.await
		.unwrap_or_else(|_| "<state>".to_string());
	info!("OnRun: {name}");
	let out = match cx.caller.get_cloned::<GoTo>().await {
		Ok(GoTo(target)) => {
			cx.world().entity(target).call::<(), Outcome>(()).await?
		}
		Err(_) => Outcome::PASS,
	};
	info!("{name}: {out:?}");
	out.xok()
}

// ── scripting ────────────────────────────────────────────────

/// `<NumberScript script="input + 1"/>` - a leaf whose behaviour is a
/// user-authored JavaScript transform of a number. On call it runs the script
/// over a seed input, logs the result, then passes. The source is visible in the
/// markup so the script *is* the behaviour, not Rust glue hiding it.
///
/// A typed `Script::<i64, i64>` cannot be authored in BSX (no generic syntax),
/// so this small front-end carries the source as a `script` attribute and lowers
/// to the typed script plus the call/log glue.
#[cfg(feature = "scripting")]
#[template]
pub fn NumberScript(#[prop(into)] script: String) -> impl Bundle {
	let action = Script::<i64, i64>::quickjs(script).into_action();
	Action::<(), Outcome>::new_async(move |cx: ActionContext| {
		let action = action.clone();
		async move {
			let result = cx.caller.call_detached(action, 41).await?;
			info!("number script result: {result}");
			Outcome::PASS.xok()
		}
	})
}

/// `<TextScript script='"hello " + input'/>` - the string counterpart of
/// [`NumberScript`]: a user-authored JavaScript transform of text, source
/// visible in markup. Runs over a seed string, logs the result, then passes.
#[cfg(feature = "scripting")]
#[template]
pub fn TextScript(#[prop(into)] script: String) -> impl Bundle {
	let action = Script::<String, String>::quickjs(script).into_action();
	Action::<(), Outcome>::new_async(move |cx: ActionContext| {
		let action = action.clone();
		async move {
			let result =
				cx.caller.call_detached(action, "world".to_string()).await?;
			info!("text script result: {result}");
			Outcome::PASS.xok()
		}
	})
}

// ── plugin ───────────────────────────────────────────────────

/// Registers the action-example actions and components, so a `main.bsx`
/// declaring `<Greet{name:".."}>`, `<Patrol/>`, `<AttackPlayer{..}>`, ...
/// resolves.
pub struct ActionExamplesPlugin;

impl Plugin for ActionExamplesPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<RandomSource>()
			// hello_world
			.register_type::<Greet>()
			// simple_action
			.register_type::<SayHello>()
			// long_running
			.register_type::<Patrol>()
			// utility_ai / malenia
			.register_type::<Health>()
			.register_type::<HealingPotions>()
			.register_type::<AttackTarget>()
			.register_type::<TryHealSelf>()
			.register_type::<AttackPlayer>()
			.register_type::<RandomScore>()
			// state_machine
			.register_type::<GoTo>()
			.register_type::<StateAction>();
		// scripting (only with the QuickJS runtime linked)
		#[cfg(feature = "scripting")]
		app.register_template::<NumberScript>()
			.register_template::<TextScript>();
	}
}
