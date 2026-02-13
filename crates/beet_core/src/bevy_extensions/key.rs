use bevy::input::keyboard::Key;
/// Extension trait for [`Key`] to add convenient constructors.
#[extend::ext(name=KeyExt)]
pub impl Key {
	/// Creates a [`Key`] from a single character.
	fn character(char: char) -> Key { Key::Character(char.to_string().into()) }
}
