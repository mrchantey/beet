//! Declarative `#[template]` forms of the imperative 3d scene setup
//! ([`lighting_3d`](super::lighting_3d), [`ground_3d`](super::ground_3d)), so a
//! scene `.bsx` names `<Lighting3d/>` / `<Ground3d/>` instead of a Rust `Startup`
//! system. Registered by [`beet_example_plugin`](crate::prelude::beet_example_plugin).
use crate::beet::prelude::*;
use crate::components::KeyboardController;
use crate::components::WrapAround;
use crate::components::spawn_ui_terminal;
use beet_core::prelude::*;
use bevy::light::CascadeShadowConfigBuilder;
use std::f32::consts::PI;

/// A shadow-casting directional key light angled like late afternoon, with the
/// cascade config the imperative [`lighting_3d`](super::lighting_3d) used. The data
/// form of that scene system, so `<Lighting3d/>` lights a `.bsx` scene.
#[template]
pub fn Lighting3d() -> impl Bundle {
	(
		DirectionalLight {
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(
			EulerRot::ZYX,
			0.0,
			1.0,
			-PI / 4.,
		)),
		CascadeShadowConfigBuilder {
			first_cascade_far_bound: 20.0,
			maximum_distance: 40.0,
			..default()
		}
		.build(),
	)
}

/// A 100x100 muted-green ground plane, the data form of the imperative
/// [`ground_3d`](super::ground_3d) scene system. A `#[template(system)]` since the
/// mesh + material are assets minted at build time, so `<Ground3d/>` floors a scene.
#[template(system)]
pub fn Ground3d(
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) -> impl Bundle {
	(
		Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(50.)))),
		MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
	)
}

/// Loads a glb/gltf scene by path and spawns its model as children, the markup
/// form of `WorldAssetRoot(assets.load(path))`, eg
/// `<WorldScene src="misc/fox.glb#Scene0" scale=0.1/>`. A `#[template(system)]`
/// loading through [`BuildAssets`] so `LoadTemplate` defers until the scene loads.
/// Named `WorldScene` (not `Scene`) to avoid bevy's `Scene` + beet's `Scene` trait.
///
/// `x`/`y`/`z`/`scale` set the model's transform in Rust because `Transform`
/// cannot be a markup spread (beet registers two `Transform` short paths, so it
/// resolves ambiguously) and a `Vec3` template prop coerces as a string. Other
/// markers (eg `FollowCursor3d`) still spread normally.
#[template(system)]
pub fn WorldScene(
	#[prop(into)] src: String,
	#[prop(default)] x: f32,
	#[prop(default)] y: f32,
	#[prop(default)] z: f32,
	#[prop(default = 1.0_f32)] scale: f32,
	mut assets: BuildAssets,
) -> impl Bundle {
	(
		WorldAssetRoot(assets.load::<WorldAsset>(src)),
		Transform::from_xyz(x, y, z).with_scale(Vec3::splat(scale)),
	)
}

/// The primary application window, spawned from data: the render binary links winit
/// as a *capability*, so it opens a window only when a scene asks for one. A scene
/// `.bsx` declares `<AppWindow/>` to open the window its `Camera3d` renders to; a
/// headless CLI/server `.bsx` omits it and runs windowless (closing the window does
/// not exit the app — see `BeetPlugins`).
#[template]
pub fn AppWindow() -> impl Bundle {
	(bevy::window::Window::default(), bevy::window::PrimaryWindow)
}

/// A terminal-style log UI, the data form of the imperative
/// [`ui_terminal`](super::ui_terminal) / [`ui_terminal_input`](super::ui_terminal_input)
/// scene systems. A `#[template(system)]` returning `()` since
/// [`spawn_ui_terminal`] spawns the UI tree top-level via [`Commands`] rather
/// than returning a bundle. `<UiTerminal/>` renders agent log output; pass
/// `input=true` (`<UiTerminal input=true/>`) to add the prompt row so the user
/// can type sentences that drive a [`TriggerWithUserSentence`] agent.
#[template(system)]
pub fn UiTerminal(
	#[prop(default)] input: bool,
	commands: Commands,
) -> impl Bundle {
	spawn_ui_terminal(commands, input);
}

/// The animated fox agent: loads `misc/fox.glb` and its idle (`Animation0`) + walk
/// (`Animation1`) `AnimationGraph` through the deferred [`BuildAssets`] path, so
/// `LoadTemplate` waits for every clip and the glb scene before the tree runs.
/// Hosts the behaviour tree as children via `<Slot/>` and carries the
/// [`AnimationGraphClips`] those children resolve clips against
/// (`<PlayAnimation clip="misc/fox.glb#Animation1"/>`). A `.bsx` authors
/// `<Foxie scale=0.1>`; the scene supplies the steer target as a spread
/// (`{SteerTarget::Entity($cheese)}`), and steering components ride the same tag.
#[template(system)]
pub fn Foxie(
	#[prop(default = 1.0_f32)] scale: f32,
	mut graphs: ResMut<Assets<AnimationGraph>>,
	mut assets: BuildAssets,
) -> impl Bundle {
	let clips = vec![
		"misc/fox.glb#Animation0".to_string(),
		"misc/fox.glb#Animation1".to_string(),
	];
	// the `<span>` hosts the behaviour-tree `<Slot/>` children; a Rust `#[template]`
	// has no neutral host element (`<Fragment>` is `.bsx`-only), and the inert
	// `Element` is harmless on a render entity.
	rsx! {
		<span {(
			WorldAssetRoot(assets.load::<WorldAsset>("misc/fox.glb#Scene0")),
			Transform::from_scale(Vec3::splat(scale)),
			build_animation_graph(&clips, &mut graphs, &mut assets),
		)}><Slot/></span>
	}
}

/// An identity-transform, visible scene root that hosts 3d scene entities as
/// children, eg `<Scene3d><Camera3d/><Ground3d/></Scene3d>`. The entry's store
/// entity has no transform, so a scene needs this root for child
/// `GlobalTransform`s to propagate. Carries the transform on its own entity
/// alongside the slot, so it emits no wrapper element.
#[template]
pub fn Scene3d() -> impl Bundle {
	rsx! { <span {(Transform::default(), Visibility::default())}><Slot/></span> }
}

/// A `Camera3d` placed at `x`/`y`/`z` looking at the origin, eg
/// `<Camera3dLookAt x=0. y=7. z=7./>` for the inverse-kinematics scene. `Transform`
/// can't be a markup spread, so the look-at transform is built here in Rust.
#[template]
pub fn Camera3dLookAt(
	#[prop(default)] x: f32,
	#[prop(default)] y: f32,
	#[prop(default)] z: f32,
) -> impl Bundle {
	(
		Camera3d::default(),
		Transform::from_xyz(x, y, z).looking_at(Vec3::ZERO, Vec3::Y),
	)
}

/// The keyboard-movable target for the inverse-kinematics arm: a blue sphere with a
/// [`KeyboardController`], the data form of the imperative `inverse_kinematics`
/// target spawn. A thin `#[template(system)]` because the sphere mints `Mesh` +
/// `StandardMaterial` assets, which are not markup values; the arm itself is authored
/// in `.bsx` (a `<WorldScene>` with `{(IkSpawner, TargetEntity::Other($target))}`)
/// pointed at this target by `bx:ref`.
#[template(system)]
pub fn IkTarget(
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) -> impl Bundle {
	(
		KeyboardController::default(),
		Transform::from_xyz(0., 1.5, 2.5).looking_to(-Vec3::Z, Vec3::Y),
		Mesh3d(meshes.add(Sphere::new(0.2))),
		MeshMaterial3d(materials.add(StandardMaterial {
			base_color: bevy::color::palettes::tailwind::BLUE_500.into(),
			unlit: true,
			..default()
		})),
	)
}

/// The 2d counterpart of [`Scene3d`]: an identity-transform, visible root that hosts
/// 2d scene entities as children, eg `<Scene2d><Camera2d/><SpaceScene/></Scene2d>`.
/// Same role as [`Scene3d`] (a transform root the entry's transformless store entity
/// lacks), kept separate so a 2d `.bsx` reads in its own dimension.
#[template]
pub fn Scene2d() -> impl Bundle {
	rsx! { <span {(Transform::default(), Visibility::default())}><Slot/></span> }
}

/// Loads an image by path into a [`Sprite`], the 2d counterpart of [`WorldScene`],
/// eg `<Sprite2d src="spaceship_pack/ship_2.png" scale=0.5/>`. A `#[template(system)]`
/// loading the image through [`BuildAssets`] so `LoadTemplate` defers until it loads;
/// `x`/`y`/`z`/`scale` set the transform in Rust (a `Vec3` markup prop coerces as a
/// string). Other markers (eg `FollowCursor2d`, `RotateToVelocity2d`) spread normally.
#[template(system)]
pub fn Sprite2d(
	#[prop(into)] src: String,
	#[prop(default)] x: f32,
	#[prop(default)] y: f32,
	#[prop(default)] z: f32,
	#[prop(default = 1.0_f32)] scale: f32,
	mut assets: BuildAssets,
) -> impl Bundle {
	(
		Sprite {
			image: assets.load(src),
			..default()
		},
		Transform::from_xyz(x, y, z).with_scale(Vec3::splat(scale)),
	)
}

/// A 2d steering agent that seeks a target forever: a sprite with the force/steer
/// bundles, `RotateToVelocity2d`, a [`Seek`] action, and the [`RunOnLoad`] that
/// kicks the action into `Running` once the deferred sprite image has loaded. The
/// data form of the imperative `seek` setup, so
/// `<SeekAgent2d src="..." {SteerTarget::Entity($planet)}/>` flies a ship at a
/// markup-referenced target. The image loads through [`BuildAssets`] so the run
/// defers until it exists; the target rides a spread that resolves the ref into the
/// `SteerTarget::Entity` variant.
#[template(system)]
pub fn SeekAgent2d(
	#[prop(into)] src: String,
	#[prop(default = 1.0_f32)] scale: f32,
	#[prop(default = 500.0_f32)] scaled_dist: f32,
	mut assets: BuildAssets,
) -> impl Bundle {
	(
		Sprite {
			image: assets.load(src),
			..default()
		},
		Transform::from_scale(Vec3::splat(scale)),
		RotateToVelocity2d,
		WrapAround,
		ForceBundle::default(),
		SteerBundle::default().scaled_dist(scaled_dist),
		// the seek runs on a behaviour child wired back to this agent via `ActionOf`,
		// so it resolves its steering components here. A self-contained `Seek` on this
		// (child-of-scene) entity would instead resolve its agent to the scene root,
		// which has no steering (see `AgentQuery`); mirrors `FetchFox`'s wiring.
		OnSpawn::new(|agent| {
			let agent_id = agent.id();
			agent.world_scope(|world| {
				world.spawn((
					ChildOf(agent_id),
					ActionOf(agent_id),
					RunOnLoad,
					Seek::default(),
				));
			});
		}),
	)
}

/// Spawns a flock of `count` boids, the data form of the imperative `flock` spawn
/// loop, so `<Flock count=300/>` fills a 2d scene with group-steering agents. Markup
/// has no loop, so the per-boid bundle (sprite + force/steer bundles + the group
/// behaviours `Separate`/`Align`/`Cohere`/`Wander` + the `CallOnSpawn` kickoff) is
/// built here in Rust. The boids spawn at world positions, so they are top-level (not
/// children of the inert template host).
#[template(system)]
pub fn Flock(
	#[prop(default = 300_usize)] count: usize,
	mut rand: ResMut<RandomSource>,
	mut assets: BuildAssets,
	mut commands: Commands,
) -> impl Bundle {
	let ship = assets.load::<Image>("spaceship_pack/ship_2.png");
	// pixel space, so scale the steering params up from their 0..1 defaults.
	const SCALE: f32 = 100.;
	for _ in 0..count {
		let position = Vec3::random_in_sphere(&mut rand.0).with_z(0.) * 500.;
		commands.spawn((
			Sprite {
				image: ship.clone(),
				..default()
			},
			Transform::from_translation(position).with_scale(Vec3::splat(0.5)),
			RotateToVelocity2d,
			WrapAround,
			ForceBundle::default(),
			SteerBundle::default().scaled_dist(SCALE),
			VelocityScalar(Vec3::new(1., 1., 0.)),
			GroupSteerAgent,
			Separate::<GroupSteerAgent>::new(1.).scaled_dist(SCALE),
			Align::<GroupSteerAgent>::new(1.).scaled_dist(SCALE),
			Cohere::<GroupSteerAgent>::new(1.).scaled_dist(SCALE),
			Wander::new(1.).scaled_dist(SCALE),
			CallOnSpawn::<(), Outcome>::default(),
		));
	}
}

/// A tiled starfield backdrop behind a 2d scene, the data form of the imperative
/// `space_scene`, so `<SpaceScene/>` backs a 2d `.bsx`. A `#[template(system)]`
/// loading the image through [`BuildAssets`] so `LoadTemplate` defers until it loads.
#[template(system)]
pub fn SpaceScene(mut assets: BuildAssets) -> impl Bundle {
	(
		Transform::from_translation(Vec3::new(0., 0., -1.))
			.with_scale(Vec3::splat(100.)),
		Sprite {
			image: assets.load("space_background/Space_Stars2.png"),
			image_mode: SpriteImageMode::Tiled {
				tile_x: true,
				tile_y: true,
				stretch_value: 0.02,
			},
			..default()
		},
	)
}
