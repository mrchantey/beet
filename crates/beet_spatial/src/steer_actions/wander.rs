use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;

/// Random walk that uses a pair of circles
/// to create somewhat cohesive movement, see [wander_impulse]
/// ## Tags
/// - [LongRunning](ActionTag::LongRunning)
/// - [MutateOrigin](ActionTag::MutateOrigin)
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun)]
pub struct Wander {
	/// The scalar to apply to the impulse
	pub scalar: f32,
	/// The distance from the agent to the outer wander circle
	pub outer_distance: f32,
	/// The radius of the outer circle
	pub outer_radius: f32,
	/// This effects the responsiveness of the wander
	pub inner_radius: f32,
	/// Representation of the last target, local to the outer circle
	// #[inspector(hidden)]
	pub last_local_target: Vec3,
}

impl Default for Wander {
	fn default() -> Self {
		Self {
			scalar: 1.,
			outer_distance: 1.,
			outer_radius: 0.5,
			inner_radius: 0.05,
			last_local_target: Vec3::ZERO,
		}
	}
}

impl Wander {
	/// Create a new wander with the given scalar
	pub fn new(scalar: f32) -> Self {
		Self {
			scalar,
			..default()
		}
	}
	/// Scale all radius and distances by this value
	pub fn scaled_dist(mut self, dist: f32) -> Self {
		self.outer_distance *= dist;
		self.outer_radius *= dist;
		self.inner_radius *= dist;
		self
	}

	/// Create a new wander with an initial forward direction
	pub fn default_forward() -> Self {
		Self {
			last_local_target: Vec3::new(0., 0., -1.),
			..default()
		}
	}
	/// Create a new wander with an initial right direction
	pub fn default_right() -> Self {
		Self {
			last_local_target: Vec3::RIGHT,
			..default()
		}
	}
}

pub(crate) fn wander(
	mut rng: ResMut<RandomSource>,
	mut agents: AgentQuery<(&Transform, &Velocity, &MaxSpeed, &mut Impulse)>,
	mut query: Query<(Entity, &mut Wander), With<Running>>,
) -> Result {
	for (action, mut wander) in query.iter_mut() {
		let (transform, velocity, max_speed, mut impulse) =
			agents.get_mut(action)?;
		**impulse += *wander_impulse(
			&transform.translation,
			&velocity,
			&mut wander,
			*max_speed,
			&mut rng.0,
		);
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();

		app.add_plugins((
			ControlFlowPlugin::default(),
			BeetSpatialPlugins::default(),
		))
		.insert_time();

		let agent = app
			.world_mut()
			.spawn((
				Transform::default(),
				ForceBundle::default(),
				SteerBundle::default(),
			))
			.with_children(|parent| {
				parent.spawn((
					OnSpawn::trigger(GetOutcome),
					ContinueRun,
					Wander::default(),
				));
			})
			.id();

		app.update();
		app.update_with_secs(1);

		app.world()
			.entity(agent)
			.get::<Transform>()
			.unwrap()
			.translation
			.xpect_not_eq(Vec3::ZERO);
	}
}
