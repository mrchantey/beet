//! # Malenia - Behavior Tree + Utility AI
//!
//! A compact enemy AI mixing both paradigms. Malenia heals herself when low
//! on health and holding potions, otherwise she attacks. Which attack she
//! uses is chosen by utility AI scoring.
//!
//! ```text
//! Repeat
//! └── Fallback                 try each until one passes
//!     ├── TryHealSelf          passes only if it could heal
//!     └── HighestScore         pick the best attack
//!         ├── Waterfoul Dance  random score
//!         └── Scarlet Aeonia   low fixed score (a desperate move)
//! ```
//!
//! The action tree is spawned under the Malenia entity, so [`AgentQuery`]
//! resolves her as the agent the actions operate on.
//!
//! Run with:
//! ```sh
//! cargo run --example malenia --features action
//! ```
use beet::prelude::*;

#[derive(Component)]
struct Health(f32);

#[derive(Component)]
struct HealingPotions(usize);

/// Heals the agent if its health is low and it has potions, else fails.
#[action]
#[derive(Component)]
async fn TryHealSelf(cx: ActionContext) -> Result<Outcome> {
	let world = cx.world();
	let agent = AgentQuery::entity_async(&world, cx.caller.id()).await;
	world
		.entity(agent)
		.with_then(|mut agent| -> Outcome {
			let potions = agent.get::<HealingPotions>().map(|p| p.0).unwrap_or(0);
			let health =
				agent.get::<Health>().map(|h| h.0).unwrap_or(f32::MAX);
			if potions == 0 || health >= 50.0 {
				return Outcome::FAIL;
			}
			if let Some(mut health) = agent.get_mut::<Health>() {
				health.0 += 30.0;
			}
			if let Some(mut potions) = agent.get_mut::<HealingPotions>() {
				potions.0 -= 1;
			}
			let remaining =
				agent.get::<Health>().map(|h| h.0).unwrap_or_default();
			cross_log!("Malenia heals herself, health now {remaining}");
			Outcome::PASS
		})
		.await
		.xok()
}

/// An attack: deals damage to the player and recoil to Malenia.
#[derive(Component, Clone)]
#[require(AttackPlayerAction)]
struct AttackPlayer {
	max_damage: f32,
	max_recoil: f32,
	player: Entity,
}

#[action(default)]
#[derive(Component)]
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
		.with_then(move |world: &mut World| -> Result<Outcome> {
			let damage: f32 = world
				.resource_mut::<RandomSource>()
				.random_range(0.0..attack.max_damage)
				.round();
			let recoil: f32 = world
				.resource_mut::<RandomSource>()
				.random_range(0.0..attack.max_recoil)
				.round();
			cross_log!("Malenia attacks with {name}");

			let player_hp = {
				let mut player_health = world
					.get_mut::<Health>(attack.player)
					.ok_or_else(|| bevyhow!("player has no Health"))?;
				player_health.0 -= damage;
				player_health.0
			};
			cross_log!("Player takes {damage} damage, health now {player_hp}");

			let malenia_hp = {
				let mut malenia_health = world
					.get_mut::<Health>(malenia)
					.ok_or_else(|| bevyhow!("Malenia has no Health"))?;
				malenia_health.0 -= recoil;
				malenia_health.0
			};
			cross_log!(
				"Malenia takes {recoil} recoil, health now {malenia_hp}"
			);

			if player_hp <= 0.0 {
				cross_log!("You lose - Malenia, Blade of Miquella, stands");
				return Outcome::FAIL.xok();
			}
			if malenia_hp <= 0.0 {
				cross_log!("You win - 'Your strength, extraordinary...'");
				return Outcome::FAIL.xok();
			}
			Outcome::PASS.xok()
		})
		.await
}

/// A [`ScoreProvider`] returning a fresh random score on each evaluation.
fn random_score() -> ScoreProvider<()> {
	ScoreProvider(Action::<(), Score>::new_system(
		|_: In<ActionContext>, mut rng: ResMut<RandomSource>| -> Result<Score> {
			Score(rng.random::<f32>()).xok()
		},
	))
}

#[beet::main]
async fn main() -> Result {
	let mut world = AsyncPlugin::world();
	world.init_resource::<RandomSource>();

	let player = world.spawn((Name::new("Elden Lord"), Health(100.0))).id();

	let outcome = world
		.spawn((
			Name::new("Malenia"),
			Health(100.0),
			HealingPotions(2),
			Repeat::new(),
			children![(
				Name::new("round"),
				Fallback::new(),
				children![
					(Name::new("Try Heal Self"), TryHealSelf),
					(
						Name::new("Attack"),
						HighestScore::new(),
						children![
							(
								Name::new("Waterfoul Dance"),
								random_score(),
								AttackPlayer {
									max_damage: 15.0,
									max_recoil: 30.0,
									player,
								},
							),
							(
								Name::new("Scarlet Aeonia"),
								ScoreProvider::<()>::fixed(Score(0.05)),
								AttackPlayer {
									max_damage: 10_000.0,
									max_recoil: 10.0,
									player,
								},
							),
						],
					),
				],
			)],
		))
		.call::<(), Outcome>(())
		.await?;
	cross_log!("battle over: {outcome:?}");
	Ok(())
}
