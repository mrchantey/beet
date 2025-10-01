use crate::prelude::*;
use beet_flow::prelude::*;
use beet_core::prelude::*;
use bevy::scene::SceneInstanceReady;
use std::f32::consts::FRAC_PI_2;

/// Hooks up the parts of the robot arm gltf scene
/// to the inverse kinematics system.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct IkSpawner;

/// Registers the observer for the IkSpawner component.
pub fn ik_spawner_plugin(app: &mut App) {
	app.world_mut()
		.register_component_hooks::<IkSpawner>()
		.on_add(|mut world, cx| {
			world
				.commands()
				.entity(cx.entity)
				.remove::<IkSpawner>()
				.observe(ik_spawner);
		});
	app.register_type::<IkSpawner>();
}

fn ik_spawner(
	trigger: On<SceneInstanceReady>,
	mut commands: Commands,
	child_nodes_query: Query<(Entity, &Name, &Transform, &Children)>,
	children_query: Query<&Children>,
	query: Populated<(Entity, &Transform, &Children, &TargetEntity)>,
) {
	let Ok((scene_root_entity, transform, scene_root_children, target_entity)) =
		query.get(trigger.target())
	else {
		return;
	};
	let Some(gltf_root) = scene_root_children.first() else {
		return;
	};
	let Ok(children) = children_query.get(*gltf_root) else {
		return;
	};

	let Some(arm_root) = children
		.iter()
		.find_map(|entity| find_by_name(&child_nodes_query, entity, "ArmRoot"))
	else {
		return;
	};

	let Some(items) =
		map_names_to_query_entries(&child_nodes_query, &arm_root.3, vec![
			"Base", "Segment1", "Segment2", "Segment3", "Gripper",
		])
	else {
		return;
	};

	let base = items[0];
	let segment1 = items[1];
	let segment2 = items[2];
	let segment3 = items[3];
	let gripper = items[4];

	// hack until globaltransform calculated in sceneinstanceready
	let scale = transform.scale.x;

	// let base_to_segment1 = segment1.2.translation - base.2.translation;
	let segment1_to_segment2 = segment2.2.translation.length() * scale;
	let segment2_to_segment3 = segment3.2.translation.length() * scale;
	let segment3_to_gripper = gripper.2.translation.length() * scale;

	let ik = IkArm4Dof::new(
		FRAC_PI_2,
		IkSegment::DEG_360.with_len(segment1_to_segment2),
		IkSegment::DEG_360.with_len(segment2_to_segment3),
		IkSegment::DEG_360.with_len(segment3_to_gripper),
	);

	let TargetEntity::Other(target) = target_entity else {
		unimplemented!();
	};

	let ik_transforms = IkArm4DofTransforms::new(
		ik, *target, base.0, segment1.0, segment2.0, segment3.0,
	);

	commands.entity(scene_root_entity).insert(ik_transforms);

	// commands.entity(phone.0).with_child((
	// 	Name::new("Phone Texture"),
	// 	Transform::from_xyz(0., 0.1, 0.).looking_to(Dir3::Z, Dir3::Y),
	// 	Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(0.9)))),
	// 	MeshMaterial3d(render_texture.0.clone()),
	// ));
}

/// Provided a list of names, each being a child of the previous,
/// returns that list of entities.
fn map_names_to_query_entries<'a>(
	query: &'a Query<(Entity, &Name, &Transform, &Children)>,
	children: &Children,
	names: Vec<&str>,
) -> Option<Vec<(Entity, &'a Name, &'a Transform, &'a Children)>> {
	let mut children = children;
	names
		.into_iter()
		.map(|name| {
			let Some(entry) = children
				.iter()
				.find_map(|child| find_by_name(query, child, name))
			else {
				return None;
			};
			children = &entry.3;
			Some(entry)
		})
		.collect()
}


fn find_by_name<'a>(
	query: &'a Query<(Entity, &Name, &Transform, &Children)>,
	entity: Entity,
	name: &str,
) -> Option<(Entity, &'a Name, &'a Transform, &'a Children)> {
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
