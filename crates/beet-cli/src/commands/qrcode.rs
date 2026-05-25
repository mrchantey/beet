use beet::prelude::*;
use image::Rgb;
use qrcode::QrCode as QrCodeGenerator;

/// Generates a QR code PNG.
///
/// `--input` is the encoded text/url, `--output` the file (default
/// `qrcode.png`), and `--light`/`--dark` set the colors as `r,g,b`.
#[action]
#[derive(Component)]
pub async fn QrCode(parts: RequestParts) -> Result<String> {
	let input = parts
		.get_param("input")
		.ok_or_else(|| bevyhow!("qrcode requires --input"))?;
	let output = parts.get_param("output").unwrap_or("qrcode.png");
	let light = parse_rgb(parts.get_param("light").unwrap_or("255,255,255"))?;
	let dark = parse_rgb(parts.get_param("dark").unwrap_or("0,0,0"))?;

	let image = QrCodeGenerator::new(input)?
		.render::<Rgb<u8>>()
		.dark_color(Rgb(dark))
		.light_color(Rgb(light))
		.build();
	image.save(output)?;
	Ok(format!("wrote qr code to {output}"))
}

/// Parses an `r,g,b` triple into an RGB byte array.
fn parse_rgb(value: &str) -> Result<[u8; 3]> {
	let parts: Vec<&str> = value.split(',').collect();
	if parts.len() != 3 {
		bevybail!("expected color as `r,g,b`, got `{value}`");
	}
	Ok([
		parts[0].trim().parse()?,
		parts[1].trim().parse()?,
		parts[2].trim().parse()?,
	])
}
