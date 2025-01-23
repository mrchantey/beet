//! A moderate example of beet demonstrating a simple enemy behavior
//! with a mix of utility ai and behavior tree paradigms.
//! 
//! The enemy, Malenia, will try to heal herself if her health is below 50% and she has potions.
//! Otherwise she will attack the player.
//! 
//! https://eldenring.wiki.fextralife.com/Malenia+Blade+of+Miquella
use beet::prelude::*;
use bevy::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn main() {
	println!("🙂\tMalenia says: 'I dreamt for so long..'\n");

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		BeetDefaultPlugins,
		// here we register all our actions that are used
		ActionPlugin::<(
			RandomScoreProvider, 
			AttackPlayer, 
			TryHealSelf
		)>::default(),
	))
	.add_systems(Update, health_handler)
	.insert_resource(RandomSource::default());

	app.world_mut()
		.spawn((Name::new("Elden Lord"), Health::default()));
	app.world_mut()
		// this is our agent, here the behavior (FallbackFlow) is attached directly
		// but it could be a child or completely seperate
		.spawn((
			Name::new("Malenia"),
			Health::default(),
			HealingPotions(2),
			FallbackFlow::default(),
			RepeatFlow::default(),
		))
		// actions can be declared inline if they have no parameters
		.observe(|_: Trigger<OnRun>| {
			println!("🙂\tMalenia is thinking..");
		})
		.with_children(|behavior_root| {
			// A common behavior tree pattern is to 'try' actions, ie try to heal self
			// if low health and has potions, else attack player
			behavior_root.spawn((
				Name::new("Try Heal Self"),
				TryHealSelf,
				TargetEntity(behavior_root.parent_entity()),
			));

			// if TryHeal fails the tree will 'fallback' to the next action
			// lets use utility ai to determine which attack to use
			behavior_root
				.spawn((Name::new("Attack"), ScoreFlow::default()))
				.with_children(|attack_root| {
					attack_root.spawn((
						Name::new("Waterfoul Dance"),
						// swap this out for a more advanced score provider based on
						// player health, distance, etc
						RandomScoreProvider::default(),
						AttackPlayer{
							max_damage: 15.0,
							max_recoil: 30.0,	
						},
						EndOnRun::success(),
					));
					attack_root.spawn((
						// pretty much doomed if she decides to use this
						// so lets give it a low score. This is a 5% chance because the alternative
						// is a random value between 0 and 1 
						Name::new("Scarlet Aeonia"),
						ScoreProvider::new(0.05),
						AttackPlayer{
							max_damage: 10_000.0,
							max_recoil: 10.0,	
						},
						EndOnRun::success(),
					));
				});
		})
		.trigger(OnRun);
	app.run();
}


#[derive(Component, Action, Reflect)]
#[observers(attack_player)]
struct AttackPlayer{
	max_damage: f32,
	max_recoil: f32,
}

fn attack_player(
	trigger: Trigger<OnRun>,
	attacks: Query<(&AttackPlayer,&Name)>,
	mut query: Query<(&mut Health, &Name)>,
	mut random_source: ResMut<RandomSource>,
) {
	let (attack,attack_name) = attacks
		.get(trigger.entity())
		.expect(expect_action::TARGET_MISSING);
	println!("⚔️  \tMalenia attacks with {}", attack_name);
	
	for (mut health, name) in query.iter_mut() {
		if name.as_str() == "Malenia" {
			let damage:f32 = random_source.0.gen_range(0.0..attack.max_recoil).round();
			health.0 -= damage;
			println!("⚔️  \tMalenia takes {} recoil damage, current health: {}", damage,health.0);
		} else {
			let damage:f32 = random_source.0.gen_range(0.0..attack.max_damage).round();
			health.0 -= damage;
			println!("⚔️  \tPlayer takes {} damage, current health: {}", damage, health.0);
		}
	}
	println!();
}


fn health_handler(query: Query<(&Health, &Name), Changed<Health>>) {
	for (health, name) in query.iter() {
		if health.0 > 0. {
			continue;
		} else if name.as_str() == "Malenia" {
			println!("🙂\tMalenia says: 'Your strength, extraordinary...'\n✅\tYou win!");
		} else {
			println!("🙂\tMalenia says: 'I am Malenia. Blade of Miquella'\n❌\tYou lose");
		}
		std::process::exit(0);
	}
}


///https://bevyengine.org/examples/math/random-sampling/
#[derive(Resource)]
pub struct RandomSource(ChaCha8Rng);

impl Default for RandomSource {
	fn default() -> Self {
		let rng = ChaCha8Rng::from_entropy();
		// let rng = ChaCha8Rng::seed_from_u64(123412341234);
		Self(rng)
	}
}

#[derive(Component)]
struct Health(f32);
#[derive(Component)]
struct HealingPotions(usize);

impl Default for Health {
	fn default() -> Self { Self(100.0) }
}

#[derive(Component, Action, Reflect)]
#[observers(provide_random_score)]
struct RandomScoreProvider {
	pub scalar: f32,
	pub offset: f32,
}

impl Default for RandomScoreProvider {
	fn default() -> Self {
		Self {
			scalar: 1.0,
			offset: 0.0,
		}
	}
}


fn provide_random_score(
	trigger: Trigger<RequestScore>,
	mut commands: Commands,
	mut random_source: ResMut<RandomSource>,
	query: Query<(&RandomScoreProvider, &Parent)>,
) {
	let (score_provider, parent) = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	let rnd: f32 = random_source.0.gen();

	commands.entity(parent.get()).trigger(OnChildScore::new(
		trigger.entity(),
		rnd * score_provider.scalar + score_provider.offset,
	));
}


#[derive(Component, Action, Reflect)]
#[observers(try_heal_self)]
struct TryHealSelf;

fn try_heal_self(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	target_agents: Query<&TargetEntity>,
	mut query: Query<(&mut Health, &mut HealingPotions)>,
) {
	let target_agent = target_agents
		.get(trigger.entity())
		.expect(expect_action::TARGET_MISSING);
	let (mut health, mut potions) = query
		.get_mut(target_agent.0)
		.expect(expect_action::TARGET_MISSING);

	if health.0 < 50.0 && potions.0 > 0 {
		health.0 += 30.;
		potions.0 -= 1;
		println!("💊\tMalenia heals herself, current health: {}\n",health.0);
		commands
			.entity(trigger.entity())
			.trigger(OnRunResult::success());
	} else {
		// we didnt do anything so action was a failure
		commands
			.entity(trigger.entity())
			.trigger(OnRunResult::failure());
	}
}
