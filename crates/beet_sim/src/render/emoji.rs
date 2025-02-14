use bevy::prelude::*;
#[cfg(feature = "bevyhub")]
use bevyhub::prelude::*;


#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct Emoji {
	/// an all UPPERCASE representing the utf8 emoji, ie "1F600" 😀
	hexcode: String,
}

pub fn emoji_plugin(app: &mut App) {
	app.register_type::<Emoji>()
		.world_mut()
		.register_component_hooks::<Emoji>()
		.on_add(|mut world, entity, _| {
			let hexcode =
				world.get::<Emoji>(entity).unwrap().hexcode().to_string();

			world
				.commands()
				.entity(entity)
				.insert(Emoji::bundle(&hexcode));
		});
}

impl Emoji {
	pub fn new(hexcode: &str) -> Self {
		Self {
			hexcode: hexcode.to_uppercase(),
		}
	}
	pub fn hexcode(&self) -> &str { &self.hexcode }
	pub fn set_hexcode(&mut self, hexcode: &str) {
		self.hexcode = hexcode.to_uppercase();
	}

	#[cfg(not(feature = "bevyhub"))]
	#[allow(unreachable_code)]
	pub fn bundle(_hexcode: &str) -> impl Bundle {
		todo!("non bevyhub bundle");
		()
	}

	#[cfg(feature = "bevyhub")]
	pub fn bundle(hexcode: &str) -> BundlePlaceholder {
		BundlePlaceholder::Pbr {
			mesh: MeshPlaceholder::Plane3d(Plane3d::new(
				Vec3::Z,
				Vec2::splat(0.5),
			)),
			material: MaterialPlaceholder::Texture {
				path: format!("openmoji/openmoji-618x618-color/{hexcode}.png"),
				alpha_mode: AlphaMode::Blend,
				unlit: true,
			},
		}
	}
}
