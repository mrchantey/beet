use bevy::prelude::*;


#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub enum BundlePlaceholder {
	Camera2d { transform: Transform },
	Camera3d { transform: Transform },
	Sprite { path: String, transform: Transform },
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
	query: Query<(Entity, &BundlePlaceholder), Added<BundlePlaceholder>>,
) {
	for (entity, placeholder) in query.iter() {
		let mut entity_commands = commands.entity(entity);
		entity_commands.remove::<BundlePlaceholder>();

		match placeholder.clone() {
			BundlePlaceholder::Camera2d { transform } => {
				entity_commands.insert(Camera2dBundle {
					transform,
					..default()
				});
			}
			BundlePlaceholder::Camera3d { transform } => {
				entity_commands.insert(Camera3dBundle {
					transform,
					..default()
				});
			}
			BundlePlaceholder::Sprite { path, transform } => {
				entity_commands.insert(SpriteBundle {
					texture: asset_server.load(path),
					transform,
					..default()
				});
			}
		}
	}
}
