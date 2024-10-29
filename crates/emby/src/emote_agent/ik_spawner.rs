use beet::prelude::*;
use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;


#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct IkSpawner;


pub fn ik_spawner(
	mut commands: Commands,
	mut events: EventReader<AssetEvent<Scene>>,
	// mut meshes: ResMut<Assets<Mesh>>,
	child_nodes_query: Query<(Entity, &Name, &GlobalTransform, &Children)>,
	// render_texture: Single<&HandleWrapper<StandardMaterial>, With<Camera>>,
	children_query: Query<&Children>,
	query: Populated<
		(Entity, &Children, &SceneRoot, &TargetAgent),
		With<IkSpawner>,
	>,
) {
	for ev in events.read() {
		let AssetEvent::LoadedWithDependencies { id } = ev else {
			continue;
		};

		let Some((scene_root_entity, scene_root_children, _, target)) =
			query.iter().find(|(_, _, scene, _)| scene.id() == *id)
		else {
			continue;
		};
		let Some(gltf_root) = scene_root_children.first() else {
			continue;
		};
		let Ok(children) = children_query.get(*gltf_root) else {
			continue;
		};

		let Some(arm_root) = children.iter().find_map(|entity| {
			find_by_name(&child_nodes_query, *entity, "ArmRoot")
		}) else {
			continue;
		};

		let Some(items) =
			find_by_name_recursive(&child_nodes_query, &arm_root.3, vec![
				"Base", "Segment1", "Segment2", "Segment3", "Gripper",
				"Target", "Phone",
			])
		else {
			continue;
		};

		let base = items[0];
		let segment1 = items[1];
		let segment2 = items[2];
		let segment3 = items[3];
		let gripper = items[4];
		// let target = items[5];
		// let phone = items[6];


		// let base_to_segment1 = segment1.2.translation - base.2.translation;
		let segment1_to_segment2 =
			segment2.2.translation().distance(segment1.2.translation());
		let segment2_to_segment3 =
			segment3.2.translation().distance(segment2.2.translation());
		let segment3_to_gripper =
			gripper.2.translation().distance(segment3.2.translation());

		let ik = IkArm4Dof::new(
			FRAC_PI_2,
			IkSegment::DEG_360.with_len(segment1_to_segment2),
			IkSegment::DEG_360.with_len(segment2_to_segment3),
			IkSegment::DEG_360.with_len(segment3_to_gripper),
		);
		let ik_transforms = IkArm4DofTransforms::new(
			ik, **target, base.0, segment1.0, segment2.0, segment3.0,
		);

		commands.entity(scene_root_entity).insert(ik_transforms);

		// commands.entity(phone.0).with_child((
		// 	Name::new("Phone Texture"),
		// 	Transform::from_xyz(0., 0.1, 0.).looking_to(Dir3::Z, Dir3::Y),
		// 	Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(0.9)))),
		// 	MeshMaterial3d(render_texture.0.clone()),
		// ));
	}
}

/// Provided a list of names, each being a child of the previous, returns that list of entities.
fn find_by_name_recursive<'a>(
	query: &'a Query<(Entity, &Name, &GlobalTransform, &Children)>,
	children: &Children,
	names: Vec<&str>,
) -> Option<Vec<(Entity, &'a Name, &'a GlobalTransform, &'a Children)>> {
	let mut children = children;
	names
		.into_iter()
		.map(|name| {
			let Some(entry) = children
				.iter()
				.find_map(|child| find_by_name(query, *child, name))
			else {
				return None;
			};
			children = &entry.3;
			Some(entry)
		})
		.collect()
}


fn find_by_name<'a>(
	query: &'a Query<(Entity, &Name, &GlobalTransform, &Children)>,
	entity: Entity,
	name: &str,
) -> Option<(Entity, &'a Name, &'a GlobalTransform, &'a Children)> {
	query
		.get(entity)
		.map(|entry| {
			if entry.1.as_str() == name {
				Some(entry)
			} else {
				None
			}
		})
		.ok()
		.flatten()
}
