use bevy::prelude::*;

pub fn clone_registry(src: &AppTypeRegistry, dst: &mut AppTypeRegistry) {
	let src = src.read();
	let mut dst = dst.write();
	for registration in src.iter() {
		dst.add_registration(registration.clone());
	}
}
