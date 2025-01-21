//! A basic behavior tree sequence example
use beet::prelude::*;
use bevy::prelude::*;

#[rustfmt::skip]
fn main() {
	
	let  mut app = App::new();
  app.insert_resource(BeetDebugConfig::default())
		.add_plugins((
			BeetDefaultPlugins,
			BeetDebugPlugin,
			bevy::log::LogPlugin::default()
	));
	app.world_mut()
		.spawn((
			Name::new("root"), 
			SequenceFlow,
		))
		.observe(|_:Trigger<OnRun>|{
		// actions can simply be observers	
			println!("I am Malenia Blade of Miquella");
		})
		.with_child((
			Name::new("child1"),
			EndOnRun::success(),
		))
		.with_child((
			Name::new("child2"),
			EndOnRun::success(),
		))
	.flush_trigger(OnRun);
}
use crate::prelude::*;
use bevy::prelude::*;
use rand::rngs::ThreadRng;

///https://bevyengine.org/examples/math/random-sampling/
#[derive(Resource)]
pub struct RandomSource(ChaCha8Rng);

/// A constant score provider.
#[derive(Default, Component, Action, Reflect)]
#[reflect(Default, Component)]
#[category(ActionCategory::ChildBehaviors)]
#[observers(provide_score)]
pub struct RandomScoreProvider {
	rng: ThreadRng,
}

impl RandomScoreProvider {
	pub fn new() -> Self {
		Self {
			rng: rand::thread_rng(),
		}
	}
}

fn provide_score(
	trigger: Trigger<RequestScore>,
	mut commands: Commands,
	query: Query<(&RandomScoreProvider, &Parent)>,
) {
	let (score_provider, parent) = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	commands.entity(parent.get()).trigger(OnChildScore::new(
		trigger.entity(),
		score_provider.rng.gen(),
	));
}
