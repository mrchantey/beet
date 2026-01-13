use beet::prelude::*;
use image::Rgb;
use qrcode::QrCode;



/// Build the project
#[derive(Debug, Clone)]
pub struct QrCodeCmd {
	/// Input url (positional)
	pub input: Vec<String>,
	/// Output file (-o, --output)
	pub output: std::path::PathBuf,
	/// Light color (--light)
	pub light: String,
	/// Dark color (--dark)
	pub dark: String,
}

impl Default for QrCodeCmd {
	fn default() -> Self {
		Self {
			input: Vec::new(),
			output: "qrcode.png".into(),
			light: "255,255,255".to_string(),
			dark: "0,0,0".to_string(),
		}
	}
}



impl QrCodeCmd {
	pub async fn run(self) -> Result {
		let Self {
			input,
			output,
			light,
			dark,
		} = self;
		let input = input.join(" ");

		// Parse light color
		let light_parts: Vec<&str> = light.split(',').collect();
		let light_rgb = [
			light_parts[0].parse::<u8>()?,
			light_parts[1].parse::<u8>()?,
			light_parts[2].parse::<u8>()?,
		];

		// Parse dark color
		let dark_parts: Vec<&str> = dark.split(',').collect();
		let dark_rgb = [
			dark_parts[0].parse::<u8>()?,
			dark_parts[1].parse::<u8>()?,
			dark_parts[2].parse::<u8>()?,
		];

		let code = QrCode::new(input)?;
		let image = code
			.render::<Rgb<u8>>()
			.dark_color(Rgb(dark_rgb))
			.light_color(Rgb(light_rgb))
			.build();

		// Save the image.
		image.save(output)?;

		Ok(())
	}
}
