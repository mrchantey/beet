use beet::prelude::*;
use clap::Parser;
use image::Rgb;
use qrcode::QrCode;



/// Build the project
#[derive(Debug, Clone, Parser)]
pub struct QrCodeCmd {
	/// Input url (positional)
	#[arg(value_name = "PROMPT", trailing_var_arg = true)]
	pub input: Vec<String>,
	/// Output file (-o, --output)
	#[clap(short = 'o', long = "output",
		default_value="qrcode.png",
	 value_parser = clap::value_parser!(std::path::PathBuf))]
	pub output: std::path::PathBuf,
	/// Light color (--light)
	#[clap(long = "light", default_value = "255,255,255")]
	pub light: String,
	/// Dark color (--dark)
	#[clap(long = "dark", default_value = "0,0,0")]
	pub dark: String,
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
