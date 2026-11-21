//! A raster picture's pixel size read off its header, never decoding it:
//! PNG, JPEG and GIF.
use crate::prelude::*;

/// The pixel size `bytes` declare, `None` for bytes of no format read here
/// or a header too short to hold one.
pub fn size(bytes: &[u8]) -> Option<UVec2> {
	png_size(bytes)
		.or_else(|| jpeg_size(bytes))
		.or_else(|| gif_size(bytes))
}

/// The pixel size of a PNG from its `IHDR`, `None` when `bytes` are no PNG.
pub fn png_size(bytes: &[u8]) -> Option<UVec2> {
	(bytes.len() >= 24
		&& bytes.starts_with(b"\x89PNG\r\n\x1a\n")
		&& &bytes[12..16] == b"IHDR")
		.then(|| UVec2::new(be_u32(&bytes[16..20]), be_u32(&bytes[20..24])))
}

/// The pixel size of a JPEG from its first start of frame marker.
fn jpeg_size(bytes: &[u8]) -> Option<UVec2> {
	if !bytes.starts_with(&[0xFF, 0xD8]) {
		return None;
	}
	let mut at = 2;
	while at + 9 <= bytes.len() {
		if bytes[at] != 0xFF {
			return None;
		}
		let marker = bytes[at + 1];
		// a fill byte, or a marker standing alone with no length
		if marker == 0xFF {
			at += 1;
			continue;
		}
		if matches!(marker, 0x01 | 0xD0..=0xD7) {
			at += 2;
			continue;
		}
		let length =
			u16::from_be_bytes([bytes[at + 2], bytes[at + 3]]) as usize;
		// every start of frame but the DHT, JPG and DAC markers sharing its range
		if matches!(marker, 0xC0..=0xCF)
			&& !matches!(marker, 0xC4 | 0xC8 | 0xCC)
		{
			let height = u16::from_be_bytes([bytes[at + 5], bytes[at + 6]]);
			let width = u16::from_be_bytes([bytes[at + 7], bytes[at + 8]]);
			return Some(UVec2::new(width as u32, height as u32));
		}
		at += 2 + length;
	}
	None
}

/// The pixel size of a GIF from its logical screen descriptor.
fn gif_size(bytes: &[u8]) -> Option<UVec2> {
	(bytes.len() >= 10
		&& (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")))
	.then(|| {
		UVec2::new(
			u16::from_le_bytes([bytes[6], bytes[7]]) as u32,
			u16::from_le_bytes([bytes[8], bytes[9]]) as u32,
		)
	})
}

fn be_u32(bytes: &[u8]) -> u32 {
	u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn reads_each_format() {
		let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
		png.extend(640u32.to_be_bytes());
		png.extend(480u32.to_be_bytes());
		raster_ext::size(&png).xpect_eq(Some(UVec2::new(640, 480)));
		// an APP0 segment, then a baseline frame of 8 bit samples
		let jpeg = [
			0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x4A, 0x46, 0xFF, 0xC0, 0x00,
			0x11, 0x08, 0x02, 0xD0, 0x05, 0x00, 0x03,
		];
		raster_ext::size(&jpeg).xpect_eq(Some(UVec2::new(1280, 720)));
		raster_ext::size(b"GIF89a\x20\x00\x10\x00")
			.xpect_eq(Some(UVec2::new(32, 16)));
		raster_ext::size(b"not a picture").xpect_none();
		raster_ext::png_size(b"GIF89a\x20\x00\x10\x00").xpect_none();
	}
}
