use bevy::prelude::*;



pub struct ResourceFns {
	pub insert: fn(&mut Commands, payload: &[u8]) -> bincode::Result<()>,
	pub remove: fn(&mut Commands, payload: &[u8]) -> bincode::Result<()>,
}
