use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use forky::prelude::Vec3Ext;

/// Somewhat cohesive random walk
#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Agent)]
#[systems(wander.in_set(TickSet))]
pub struct Wander {
	/// The scalar to apply to the impulse
	pub scalar: f32,
	pub outer_distance: f32,
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

	pub fn with_impulse(mut self, impulse: f32) -> Self {
		self.scalar = impulse;
		self
	}

	pub fn default_forward() -> Self {
		Self {
			last_local_target: Vec3::new(0., 0., -1.),
			..default()
		}
	}
	pub fn default_right() -> Self {
		Self {
			last_local_target: Vec3::RIGHT,
			..default()
		}
	}
}

fn wander(
	mut agents: Query<(&Transform, &Velocity, &MaxSpeed, &mut Impulse)>,
	mut query: Query<
		(&TargetAgent, &mut Wander),
		(With<Running>, With<Wander>),
	>,
) {
	for (agent, mut wander) in query.iter_mut() {
		if let Ok((transform, velocity, max_speed, mut impulse)) =
			agents.get_mut(**agent)
		{
			**impulse += *wander_impulse(
				&transform.translation,
				&velocity,
				&mut wander,
				*max_speed,
			);
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use beet_flow::prelude::*;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();

		app.add_plugins((LifecyclePlugin, MovementPlugin, SteerPlugin))
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
					TargetAgent(parent.parent_entity()),
					Running,
					Wander::default(),
				));
			})
			.id();

		app.update();
		app.update_with_secs(1);

		expect(app.world())
			.component::<Transform>(agent)?
			.map(|t| t.translation)
			.not()
			.to_be(Vec3::ZERO)?;

		Ok(())
	}
}
