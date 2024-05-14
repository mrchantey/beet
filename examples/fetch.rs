// use beet::prelude::*;
use beet::prelude::*;
use bevy::animation::RepeatAnimation;
use bevy::prelude::*;
// use example_plugin::ExamplePlugin;
use beet_examples::*;
use rand::prelude::IteratorRandom;
use std::time::Duration;

fn main() {
	App::new()
		.add_plugins(ExamplePlugin3d)
		.add_plugins(DefaultBeetPlugins)
		.add_plugins(BeetDebugPlugin::default())
		.add_systems(
			Startup,
			(setup_camera, setup_fox, setup_chat, setup_items),
		)
		.add_plugins(DialogPanelPlugin)
		.run();
}

fn setup_camera(mut commands: Commands) {
	commands.spawn(Camera3dBundle {
		transform: Transform::from_xyz(50., 30., 100.)
			.looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
		..default()
	});
}
fn setup_chat(mut npc_events: EventWriter<OnNpcMessage>) {
	npc_events.send(OnNpcMessage(what_does_the_fox_say()));
}

fn setup_fox(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut graphs: ResMut<Assets<AnimationGraph>>,
) {
	let mut graph = AnimationGraph::new();

	let anim1_clip = asset_server.load("Fox.glb#Animation0");
	let anim1_index = graph.add_clip(anim1_clip.clone(), 1.0, graph.root);
	let anim2_clip = asset_server.load("Fox.glb#Animation1");
	let anim2_index = graph.add_clip(anim2_clip.clone(), 1.0, graph.root);

	let transition_duration = Duration::from_secs_f32(0.5);

	commands
		.spawn((
			SceneBundle {
				scene: asset_server.load("Fox.glb#Scene0"),
				transform: Transform::from_scale(Vec3::splat(0.1)),
				..default()
			},
			graphs.add(graph),
			AnimationTransitions::new(),
		))
		.with_children(|parent| {
			let agent = parent.parent_entity();
			parent
				.spawn((
					Name::new("Animation Behavior"),
					Running,
					SequenceSelector,
					Repeat,
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Idle"),
						TargetAgent(agent),
						PlayAnimation::new(anim1_index)
							.repeat(RepeatAnimation::Forever),
						InsertOnAnimationEnd::new(
							anim1_clip,
							anim1_index,
							RunResult::Success,
						)
						.with_transition_duration(transition_duration),
					));
					parent.spawn((
						Name::new("Walking"),
						TargetAgent(agent),
						PlayAnimation::new(anim2_index)
							.repeat(RepeatAnimation::Count(4))
							.with_transition_duration(transition_duration),
						InsertOnAnimationEnd::new(
							anim2_clip,
							anim2_index,
							RunResult::Success,
						)
						.with_transition_duration(transition_duration),
					));
				});
		});
}


#[derive(Component)]
pub struct Item;

fn setup_items(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let scale = Vec3::splat(5.);
	let offset = 40.;
	commands.spawn((Name::new("Yellow Cube"), PbrBundle {
		mesh: meshes.add(Cuboid::default()),
		material: materials.add(Color::srgb(1., 1., 0.)),
		transform: Transform::from_xyz(-offset, scale.y * 0.5, -offset)
			.with_scale(scale),
		..default()
	}));
	commands.spawn((Name::new("Red Sphere"), PbrBundle {
		mesh: meshes.add(Sphere::default()),
		material: materials.add(Color::srgb(1., 0., 0.)),
		transform: Transform::from_xyz(offset, scale.y * 0.5, -offset)
			.with_scale(scale),
		..default()
	}));
	commands.spawn((Name::new("Green Cylinder"), PbrBundle {
		mesh: meshes.add(Cylinder::default()),
		material: materials.add(Color::srgb(0., 1., 0.)),
		transform: Transform::from_xyz(-offset, scale.y * 0.5, offset)
			.with_scale(scale),
		..default()
	}));
	commands.spawn((Name::new("Blue Torus"), PbrBundle {
		mesh: meshes.add(Torus::default()),
		material: materials.add(Color::srgb(0., 0., 1.)),
		transform: Transform::from_xyz(offset, scale.y * 0.5, offset)
			.with_scale(scale),
		..default()
	}));
}

fn what_does_the_fox_say() -> String {
	let sounds = [
		"Wa-pa-pa-pa-pa-pa-pow!",
		"Hatee-hatee-hatee-ho!",
		"Joff-tchoff-tchoffo-tchoffo-tchoff!",
		"Jacha-chacha-chacha-chow!",
		"Fraka-kaka-kaka-kaka-kow!",
		"A-hee-ahee ha-hee!",
		"A-oo-oo-oo-ooo!",
		"Ring-ding-ding-ding-dingeringeding!",
	];
	let sound = sounds.iter().choose(&mut rand::thread_rng()).unwrap();
	sound.to_string()
}
