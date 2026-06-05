use beet::prelude::*;
use image::Rgb;
use qrcode::QrCode as QrCodeGenerator;

/// Request params for the [`QrCode`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct QrCodeParams {
	/// The text/url to encode.
	#[reflect(@RequiredField)]
	input: String,
	/// The output file path, defaults to `qrcode.png`.
	output: Option<String>,
	/// Light module color as `r,g,b`, defaults to `255,255,255`.
	light: Option<String>,
	/// Dark module color as `r,g,b`, defaults to `0,0,0`.
	dark: Option<String>,
}

/// Generates a QR code PNG.
///
/// `--input` is the encoded text/url, `--output` the file (default
/// `qrcode.png`), and `--light`/`--dark` set the colors as `r,g,b`.
#[action(route = "qrcode", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<QrCodeParams>())]
pub async fn QrCode(parts: RequestParts) -> Result<String> {
	let params = parts.params().parse_reflect::<QrCodeParams>()?;
	let output = params.output.as_deref().unwrap_or("qrcode.png");
	let light = parse_rgb(params.light.as_deref().unwrap_or("255,255,255"))?;
	let dark = parse_rgb(params.dark.as_deref().unwrap_or("0,0,0"))?;

	let image = QrCodeGenerator::new(&params.input)?
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
