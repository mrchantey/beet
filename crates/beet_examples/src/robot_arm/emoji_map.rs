use bevy::prelude::*;
use bevy::utils::HashMap;

#[derive(Debug, Default, Deref, DerefMut, Resource)]
pub struct EmojiMap(pub HashMap<String, Handle<Image>>);


impl EmojiMap {
	pub const HAPPY: &str = "1F642";
	pub const BLINK: &str = "1F60A";

	pub fn list() -> Vec<&'static str> { vec![Self::HAPPY, Self::BLINK] }

	pub fn new(asset_server: &AssetServer) -> Self {
		Self(
			Self::list()
				.into_iter()
				.map(|hexcode| {
					(
						hexcode.to_string(),
						asset_server.load(Self::file_path(hexcode)),
					)
				})
				.collect(),
		)
	}

	pub fn file_path(hexcode: &str) -> String {
		format!("openmoji/openmoji-618x618-color/{}.png", hexcode)
	}
}
