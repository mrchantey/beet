use bevy::color::Color;
// `f32::round` is std-only; `bevy::math::ops::round` uses libm under no_std.
use bevy::math::ops;


/// A simple struct representing an sRGBA color with 8 bits per channel.
pub struct SrgbaU8 {
	/// The red component
	pub red: u8,
	/// The green component
	pub green: u8,
	/// The blue component
	pub blue: u8,
	/// The alpha component
	pub alpha: u8,
}

impl Into<Color> for SrgbaU8 {
	fn into(self) -> Color {
		Color::srgba_u8(self.red, self.green, self.blue, self.alpha)
	}
}
/// An extension trait for `Color` that provides additional methods.
#[extend::ext(name=ColorExt)]
pub impl Color {
	/// Converts the color to an sRGBA representation with 8 bits per channel.
	fn to_srgba_u8(&self) -> SrgbaU8 {
		let srgba = self.to_srgba();
		SrgbaU8 {
			red: ops::round(srgba.red * 255.0) as u8,
			green: ops::round(srgba.green * 255.0) as u8,
			blue: ops::round(srgba.blue * 255.0) as u8,
			alpha: ops::round(srgba.alpha * 255.0) as u8,
		}
	}
}
