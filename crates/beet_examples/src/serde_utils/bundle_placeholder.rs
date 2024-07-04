use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;


#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub enum BundlePlaceholder {
	Camera2d { transform: Transform },
	Camera3d { transform: Transform },
}

impl BundlePlaceholder {
	pub fn apply(self, commands: &mut EntityCommands) {
		match self {
			BundlePlaceholder::Camera2d { transform } => {
				commands.insert(Camera2dBundle {
					transform,
					..default()
				});
			}
			BundlePlaceholder::Camera3d { transform } => {
				commands.insert(Camera3dBundle {
					transform,
					..default()
				});
			}
		}
	}
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
	mut commands: Commands,
	query: Query<(Entity, &BundlePlaceholder), Added<BundlePlaceholder>>,
) {
	for (entity, placeholder) in query.iter() {
		let mut entity_commands = commands.entity(entity);
		entity_commands.remove::<BundlePlaceholder>();

		placeholder.clone().apply(&mut entity_commands);
	}
}
