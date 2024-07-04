use bevy::prelude::*;


#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub enum BundlePlaceholder {
	Camera2d,
	Camera3d,
	Sprite(String),
	Scene(String),
}

#[derive(Debug, Default)]
pub struct BundlePlaceholderPlugin;

impl Plugin for BundlePlaceholderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(PreUpdate, init_bundle)
			.register_type::<BundlePlaceholder>();
	}
}


fn init_bundle(
	asset_server: Res<AssetServer>,
	mut commands: Commands,
	query: Query<
		(Entity, Option<&Transform>, &BundlePlaceholder),
		Added<BundlePlaceholder>,
	>,
) {
	for (entity, transform, placeholder) in query.iter() {
		let mut entity_commands = commands.entity(entity);
		entity_commands.remove::<BundlePlaceholder>();
		let transform = transform.cloned().unwrap_or_default();

		match placeholder.clone() {
			BundlePlaceholder::Camera2d => {
				entity_commands.insert(Camera2dBundle {
					transform,
					..default()
				});
			}
			BundlePlaceholder::Camera3d => {
				entity_commands.insert(Camera3dBundle {
					transform,
					..default()
				});
			}
			BundlePlaceholder::Sprite(path) => {
				entity_commands.insert(SpriteBundle {
					texture: asset_server.load(path),
					transform,
					..default()
				});
			}
			BundlePlaceholder::Scene(path) => {
				entity_commands.insert(SceneBundle {
					scene: asset_server.load(path),
					transform,
					..default()
				});
			}
		}
	}
}
