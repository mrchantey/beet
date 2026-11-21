//! [`OfficePicture`], a picture as every Office reader and writer carries it,
//! by the [module docs](super)' Pictures section.
use super::office_style::OfficeUnits;
use beet_core::prelude::*;

/// A picture an `img` shows: its bytes when its `src` is a `data:` URL,
/// else the link it names, and the size it is drawn at.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OfficePicture {
	/// Where the picture's bytes are.
	pub source: PictureSource,
	/// Its size in CSS px, fitted to the box it is drawn in.
	pub size: Vec2,
	/// Its `alt`.
	pub alt: String,
}

/// Where a picture's bytes are.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PictureSource {
	/// Carried in the tree, a `data:` URL's.
	Embedded(DataUrl),
	/// Elsewhere, at its `src`, which a writer links.
	Linked(String),
}

impl OfficePicture {
	/// The size a picture is drawn at when nothing says, a square of 96 px.
	const FALLBACK_SIZE: Vec2 = Vec2::splat(96.);

	/// The `img` a reader reads a picture as: its bytes as a `data:` URL of
	/// `content_type`, its size in px from `emu`, a width and height in EMU.
	pub fn read_embedded(
		bytes: &[u8],
		content_type: &str,
		emu: Option<UVec2>,
		alt: &str,
	) -> super::read_element::ReadElement {
		Self::read(
			DataUrl::new(content_type, bytes.to_vec()).to_string(),
			emu,
			alt,
		)
	}

	/// The `img` a reader reads a picture at `src` as, its size in px from
	/// `emu`.
	pub fn read(
		src: String,
		emu: Option<UVec2>,
		alt: &str,
	) -> super::read_element::ReadElement {
		let mut img = super::read_element::ReadElement::new("img")
			.with_attribute("src", src)
			.with_attribute("alt", alt.to_string());
		if let Some(emu) = emu.filter(|emu| emu.x > 0 && emu.y > 0) {
			let px = |emu: u32| {
				((emu as f64 * OfficeUnits::PX_PER_INCH as f64 / 914400.)
					.round() as u32)
					.max(1)
					.to_string()
			};
			img = img
				.with_attribute("width", px(emu.x))
				.with_attribute("height", px(emu.y));
		}
		img
	}

	/// The picture an `img` writes as: its `src` decoded when a `data:` URL,
	/// sized by `resolved` (its resolved width and height in px), else its
	/// `width` and `height` attributes, else its native size, the other side
	/// kept at its aspect, and scaled down to fit `bounds`.
	pub fn write(
		src: &str,
		alt: &str,
		attributes: Vec2,
		resolved: Vec2,
		bounds: Vec2,
	) -> Self {
		let source = match DataUrl::is_data_url(src) {
			true => DataUrl::parse(src)
				.map(PictureSource::Embedded)
				.unwrap_or_else(|_| PictureSource::Linked(src.to_string())),
			false => PictureSource::Linked(src.to_string()),
		};
		let native = match &source {
			PictureSource::Embedded(url) => Self::native_size(&url.bytes),
			PictureSource::Linked(_) => None,
		};
		// each side from the first that says it, the other by aspect
		let pick = |index: usize| {
			[resolved, attributes]
				.iter()
				.map(|size| size[index])
				.find(|side| *side > 0.)
		};
		let aspect = native
			.map(|native| native.x / native.y)
			.or_else(|| match (pick(0), pick(1)) {
				(Some(width), Some(height)) => Some(width / height),
				_ => None,
			})
			.unwrap_or(1.);
		let size = match (pick(0), pick(1)) {
			(Some(width), Some(height)) => Vec2::new(width, height),
			(Some(width), None) => Vec2::new(width, width / aspect),
			(None, Some(height)) => Vec2::new(height * aspect, height),
			(None, None) => native.unwrap_or(Self::FALLBACK_SIZE),
		};
		Self {
			source,
			size: Self::fit(size, bounds),
			alt: alt.to_string(),
		}
	}

	/// `size` scaled down at its aspect to fit `bounds`, a side of `bounds`
	/// at or below 0 not bounding.
	pub fn fit(size: Vec2, bounds: Vec2) -> Vec2 {
		let scale = [0, 1]
			.into_iter()
			.filter(|index| {
				bounds[*index] > 0. && size[*index] > bounds[*index]
			})
			.map(|index| bounds[index] / size[index])
			.fold(1f32, f32::min);
		size * scale
	}

	/// The size in EMU a picture is written at.
	pub fn emu(&self) -> (i64, i64) {
		(OfficeUnits::emu(self.size.x), OfficeUnits::emu(self.size.y))
	}

	/// The extension a picture part of `media_type` is named with.
	pub fn extension(media_type: &str) -> &'static str {
		match media_type {
			"image/png" => "png",
			"image/jpeg" | "image/jpg" => "jpeg",
			"image/gif" => "gif",
			"image/bmp" => "bmp",
			"image/tiff" => "tiff",
			"image/svg+xml" => "svg",
			"image/webp" => "webp",
			"image/x-emf" | "image/emf" => "emf",
			"image/x-wmf" | "image/wmf" => "wmf",
			_ => "bin",
		}
	}

	/// The pixel size a PNG, GIF or JPEG's header declares, as CSS px.
	fn native_size(bytes: &[u8]) -> Option<Vec2> {
		let be16 = |at: usize| {
			bytes
				.get(at..at + 2)
				.map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
		};
		let size = match bytes {
			[0x89, b'P', b'N', b'G', ..] => {
				let be32 = |at: usize| {
					bytes.get(at..at + 4).map(|quad| {
						u32::from_be_bytes([quad[0], quad[1], quad[2], quad[3]])
					})
				};
				Vec2::new(be32(16)? as f32, be32(20)? as f32)
			}
			[b'G', b'I', b'F', ..] => {
				let le16 = |at: usize| {
					bytes
						.get(at..at + 2)
						.map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
				};
				Vec2::new(le16(6)? as f32, le16(8)? as f32)
			}
			[0xFF, 0xD8, ..] => {
				// walk the segments to the first start of frame
				let mut at = 2;
				loop {
					if *bytes.get(at)? != 0xFF {
						return None;
					}
					let marker = *bytes.get(at + 1)?;
					let length = be16(at + 2)? as usize;
					if matches!(marker, 0xC0..=0xCF)
						&& !matches!(marker, 0xC4 | 0xC8 | 0xCC)
					{
						break Vec2::new(
							be16(at + 7)? as f32,
							be16(at + 5)? as f32,
						);
					}
					at += 2 + length;
				}
			}
			_ => return None,
		};
		(size.x > 0. && size.y > 0.).then_some(size)
	}
}

#[cfg(test)]
pub(crate) mod test {
	use super::*;

	/// A 4 by 2 pixel PNG's leading bytes, as much as its size needs.
	pub(crate) fn png() -> Vec<u8> {
		let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
		bytes.extend([0, 0, 0, 13]);
		bytes.extend(b"IHDR");
		bytes.extend(4u32.to_be_bytes());
		bytes.extend(2u32.to_be_bytes());
		bytes.extend([8, 6, 0, 0, 0]);
		bytes
	}

	/// A picture is sized by what says first, the other side by its aspect,
	/// and fitted to its box.
	#[beet_core::test]
	fn sizes_a_picture() {
		let src = DataUrl::new("image/png", png()).to_string();
		let size = |attributes: Vec2, resolved: Vec2, bounds: Vec2| {
			OfficePicture::write(&src, "", attributes, resolved, bounds).size
		};
		size(Vec2::ZERO, Vec2::ZERO, Vec2::ZERO).xpect_eq(Vec2::new(4., 2.));
		size(Vec2::new(40., 0.), Vec2::ZERO, Vec2::ZERO)
			.xpect_eq(Vec2::new(40., 20.));
		size(Vec2::new(40., 0.), Vec2::new(0., 10.), Vec2::ZERO)
			.xpect_eq(Vec2::new(40., 10.));
		size(Vec2::new(1000., 0.), Vec2::ZERO, Vec2::new(500., 0.))
			.xpect_eq(Vec2::new(500., 250.));
		OfficePicture::write(
			"https://example.com/a.png",
			"",
			Vec2::ZERO,
			Vec2::ZERO,
			Vec2::ZERO,
		)
		.source
		.xpect_eq(PictureSource::Linked("https://example.com/a.png".into()));
	}
}
