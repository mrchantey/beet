use bevy::prelude::*;

pub struct ReflectUtils;

impl ReflectUtils {
	pub fn short_path(val: &dyn PartialReflect) -> String {
		val.get_represented_type_info()
			.map(|i| i.type_path_table().short_path())
			.unwrap_or("unknown")
			.to_string()
	}
	pub fn name(val: &dyn PartialReflect) -> String {
		heck::AsTitleCase(Self::short_path(val)).to_string()
	}
	pub fn ident(val: &dyn PartialReflect) -> String {
		val.get_represented_type_info()
			.map(|i| i.type_path_table().ident())
			.flatten()
			.unwrap_or("unknown")
			.to_string()
	}
	pub fn ident_name(val: &dyn PartialReflect) -> String {
		heck::AsTitleCase(Self::ident(val)).to_string()
	}

	// https://github.com/bevyengine/bevy/blob/89a41bc62843be5f92b4b978f6d801af4de14a2d/crates/bevy_reflect/src/type_registry.rs#L156
	/// converts [`std::any::type_name`] into a short version
	pub fn get_short_name(name: &str) -> String {
		let mut short_name = String::new();
		{
			// A typename may be a composition of several other type names (e.g. generic parameters)
			// separated by the characters that we try to find below.
			// Then, each individual typename is shortened to its last path component.
			//
			// Note: Instead of `find`, `split_inclusive` would be nice but it's still unstable...
			let mut remainder = name;
			while let Some(index) =
				remainder.find(&['<', '>', '(', ')', '[', ']', ',', ';'][..])
			{
				let (path, new_remainder) = remainder.split_at(index);
				// Push the shortened path in front of the found character
				short_name.push_str(path.rsplit(':').next().unwrap());
				// Push the character that was found
				let character = new_remainder.chars().next().unwrap();
				short_name.push(character);
				// Advance the remainder
				if character == ',' || character == ';' {
					// A comma or semicolon is always followed by a space
					short_name.push(' ');
					remainder = &new_remainder[2..];
				} else {
					remainder = &new_remainder[1..];
				}
			}

			// The remainder will only be non-empty if there were no matches at all
			if !remainder.is_empty() {
				// Then, the full typename is a path that has to be shortened
				short_name.push_str(remainder.rsplit(':').next().unwrap());
			}
		}

		short_name
	}
}
