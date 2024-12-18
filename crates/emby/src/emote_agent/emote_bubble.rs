use crate::prelude::*;
use bevy::prelude::*;
use bevyhub::prelude::*;

#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct EmoteBubble;



pub fn spawn_emote_bubble(commands: &mut EntityCommands) -> Entity {
	commands
		.insert((Name::new("Emote Bubble"), BundlePlaceholder::Pbr {
			mesh: MeshPlaceholder::Plane3d(Plane3d::new(
				Vec3::Z,
				Vec2::new(0.5, 0.5),
			)),
			material: MaterialPlaceholder::Texture {
				path: "icons/speech-bubble.png".into(),
				alpha_mode: AlphaMode::Blend,
				unlit: true,
			},
		}))
		.with_child((
			Name::new("Emote Bubble"),
			Transform::from_xyz(0., 0.07, 0.1),
			BundlePlaceholder::Pbr {
				mesh: MeshPlaceholder::Plane3d(Plane3d::new(
					Vec3::Z,
					Vec2::splat(0.4),
				)),
				material: MaterialPlaceholder::Texture {
					path: EmojiMap::file_path(EmojiMap::HAPPY),
					alpha_mode: AlphaMode::Blend,
					unlit: true,
				},
			},
		))
		.id()
}
