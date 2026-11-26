//! [`OfficePicture`], a picture as every Office reader and writer carries it,
//! by the [module docs](super)' Pictures section.
use super::office_style::OfficeUnits;
use crate::prelude::*;
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
	/// Carried in the tree: a `data:` url's, or fetched onto its `src` as an
	/// [`InlineBlob`].
	Embedded(MediaBytes),
	/// Elsewhere, at its `src`, which a writer links.
	Linked(String),
}

/// The pictures of a tree being written.
#[derive(SystemParam)]
pub(crate) struct PictureQuery<'w, 's> {
	elements: ElementQuery<'w, 's>,
	attributes: AttributeQuery<'w, 's>,
	blobs: Query<'w, 's, &'static InlineBlob>,
}

impl PictureSource {
	/// The source `src` names: its bytes when a `data:` url, else the link.
	pub fn of(src: &str) -> Self {
		Url::coerce(src)
			.media_bytes()
			.map(Self::Embedded)
			.unwrap_or_else(|_| Self::Linked(src.to_string()))
	}
}

impl PictureQuery<'_, '_> {
	/// Each `img` among `entities` as an unsized [`OfficePicture`]: its
	/// source the [`InlineBlob`] a media resolve step left on its `src`,
	/// else what `src` names.
	pub fn gather(
		&self,
		entities: &[Entity],
	) -> HashMap<Entity, OfficePicture> {
		entities
			.iter()
			.filter_map(|entity| {
				let view = self.elements.get(*entity).ok()?;
				if view.tag() != "img" {
					return None;
				}
				let side = |key: &str| {
					view.attribute(key)
						.and_then(|value| {
							value
								.value
								.to_string()
								.trim_end_matches("px")
								.parse::<f32>()
								.ok()
						})
						.unwrap_or(0.)
				};
				let source = self
					.attributes
					.find(*entity, "src")
					.and_then(|(attribute, _)| self.blobs.get(attribute).ok())
					.map(|blob| PictureSource::Embedded(blob.bytes.clone()))
					.unwrap_or_else(|| {
						PictureSource::of(&view.attribute_string("src"))
					});
				let picture = OfficePicture::new(
					source,
					&view.attribute_string("alt"),
					Vec2::new(side("width"), side("height")),
				);
				Some((*entity, picture))
			})
			.collect()
	}
}

impl OfficePicture {
	/// The size a picture is drawn at when nothing says, a square of 96 px.
	const FALLBACK_SIZE: Vec2 = Vec2::splat(96.);

	/// The `img` a reader reads a picture as: its bytes as a `data:` URL of
	/// `content_type`, its size in px from `emu`, a width and height in EMU.
	pub fn read_embedded(
		bytes: &[u8],
		content_type: &str,
		emu: Option<Vec2>,
		alt: &str,
	) -> super::read_element::ReadElement {
		let bytes = MediaBytes::new(
			MediaType::from_content_type(content_type),
			bytes.to_vec(),
		);
		Self::read(Url::from_media_bytes(&bytes).to_string(), emu, alt)
	}

	/// The `img` a reader reads a picture at `src` as, its size in px from
	/// `emu`.
	pub fn read(
		src: String,
		emu: Option<Vec2>,
		alt: &str,
	) -> super::read_element::ReadElement {
		let mut img = super::read_element::ReadElement::new("img")
			.with_attribute("src", src)
			.with_attribute("alt", alt.to_string());
		if let Some(emu) = emu.filter(|emu| emu.cmpgt(Vec2::ZERO).all()) {
			let px = |emu: f32| {
				(OfficeUnits::px_from_emu(emu).round() as u32)
					.max(1)
					.to_string()
			};
			img = img
				.with_attribute("width", px(emu.x))
				.with_attribute("height", px(emu.y));
		}
		img
	}

	/// A picture of `source`, sized by its `img`'s `width` and `height`
	/// attributes in px, a side of 0 unsaid, until [`sized`](Self::sized).
	pub fn new(source: PictureSource, alt: &str, attributes: Vec2) -> Self {
		Self {
			source,
			size: attributes,
			alt: alt.to_string(),
		}
	}

	/// This picture sized by `resolved` (its resolved width and height in
	/// px), else its attributes, else its native size, the other side kept
	/// at its aspect, and scaled down to fit `bounds`.
	pub fn sized(&self, resolved: Vec2, bounds: Vec2) -> Self {
		let native = match &self.source {
			PictureSource::Embedded(bytes) => Self::native_size(bytes.bytes()),
			PictureSource::Linked(_) => None,
		};
		// each side from the first that says it, the other by aspect
		let pick = |index: usize| {
			[resolved, self.size]
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
			size: Self::fit(size, bounds),
			..self.clone()
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

	/// The extension a picture part of `media_type` is named with: the
	/// type's own, else its subtype, ie `emf` for `image/x-emf`.
	pub fn extension(media_type: &MediaType) -> String {
		media_type
			.extension()
			.map(str::to_string)
			.unwrap_or_else(|| {
				media_type
					.as_str()
					.rsplit('/')
					.next()
					.map(|subtype| subtype.trim_start_matches("x-"))
					.filter(|subtype| {
						!subtype.is_empty()
							&& subtype
								.chars()
								.all(|char| char.is_ascii_alphanumeric())
					})
					.unwrap_or("bin")
					.to_string()
			})
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
		let src =
			Url::from_media_bytes(&MediaBytes::new(MediaType::Png, png()))
				.to_string();
		let size = |attributes: Vec2, resolved: Vec2, bounds: Vec2| {
			OfficePicture::new(PictureSource::of(&src), "", attributes)
				.sized(resolved, bounds)
				.size
		};
		size(Vec2::ZERO, Vec2::ZERO, Vec2::ZERO).xpect_eq(Vec2::new(4., 2.));
		size(Vec2::new(40., 0.), Vec2::ZERO, Vec2::ZERO)
			.xpect_eq(Vec2::new(40., 20.));
		size(Vec2::new(40., 0.), Vec2::new(0., 10.), Vec2::ZERO)
			.xpect_eq(Vec2::new(40., 10.));
		size(Vec2::new(1000., 0.), Vec2::ZERO, Vec2::new(500., 0.))
			.xpect_eq(Vec2::new(500., 250.));
		PictureSource::of("https://example.com/a.png").xpect_eq(
			PictureSource::Linked("https://example.com/a.png".into()),
		);
	}
}
