//! A combined example demonstrating a behavior
//! that involves animation and steering.
use beet::examples::scenes;
use beet::prelude::*;

pub fn main() {
	App::new()
		.add_plugins((BeetPlugins, BeetExamplePlugins))
		.add_systems(
			Startup,
			(
				scenes::ui_terminal,
				scenes::lighting_3d,
				scenes::ground_3d,
				setup,
			),
		)
		.run();
}

fn setup(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut anim_graphs: ResMut<Assets<AnimationGraph>>,
) {
	commands.spawn((
		Name::new("Camera"),
		Camera3d::default(),
		CameraDistance {
			width: 80.,
			offset: Vec3::new(0., 20., 40.),
		},
	));

	let target = commands
		.spawn((
			Name::new("Cheese"),
			FollowCursor3d::default(),
			Transform::from_xyz(20., 0., 40.).with_scale(Vec3::splat(3.)),
			WorldAssetRoot(asset_server.load("kaykit/cheese.glb#Scene0")),
		))
		.id();

	let Foxie {
		graph_handle,
		idle_index,
		idle_clip,
		walk_index,
		walk_clip: _,
	} = Foxie::new(&asset_server, &mut anim_graphs);

	let transition_duration = Duration::from_secs_f32(0.5);

	commands.spawn((
		Name::new("Foxie"),
		Transform::from_scale(Vec3::splat(0.1)),
		WorldAssetRoot(asset_server.load("misc/fox.glb#Scene0")),
		graph_handle,
		AnimationTransitions::new(),
		RotateToVelocity3d::default(),
		ForceBundle::default(),
		SteerBundle {
			max_force: MaxForce(0.05),
			..default()
		}
		.scaled_dist(10.),
		SteerTarget::Entity(target),
		children![(
			Name::new("Behavior"),
			TriggerOnAnimationReady::run(),
			Repeat::new(),
			children![(Name::new("round"), Sequence::new(), children![
				(Name::new("Idle"), Sequence::new(), children![
					(
						Name::new("Stop Moving"),
						InsertOn::new_with_target(
							Velocity::default(),
							TargetEntity::Agent,
						),
					),
					(
						Name::new("Play Idle"),
						PlayAnimation::new(idle_index)
							.with_transition_duration(transition_duration,),
					),
					(
						Name::new("Await Idle End"),
						TriggerOnAnimationEnd::new(
							idle_clip,
							idle_index,
							Outcome::PASS,
						)
						.with_transition_duration(transition_duration),
					),
				],),
				(Name::new("Seek"), Sequence::new(), children![
					(
						Name::new("Play Walk"),
						PlayAnimation::new(walk_index)
							.repeat_forever()
							.with_transition_duration(transition_duration,),
					),
					(
						// Seek steers the agent toward the target each
						// frame while [`Running`]; EndOnArrive ends the
						// run with [`Outcome::PASS`] once the agent is
						// within radius.
						Name::new("Seek to Arrive"),
						Seek::default(),
						EndOnArrive::new(6.),
					),
				],)
			])]
		)],
	));
}
