//! `TakePhoto`: the raw photo capture, `In = ()`, `Out = MediaBytes`. Pure: no model,
//! no describe. The head client serves this `take-photo` capability (this desktop
//! binary captures the floor-photo fixtures; the browser binary serves the same route
//! from the real webcam in V3). [`InterpretPhoto`](super::InterpretPhoto) calls it.
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Capture a photo and return its bytes (the raw image, no description).
#[action(route = "take-photo")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn TakePhoto(cx: ActionContext<()>) -> Result<MediaBytes> {
	capture(&cx.caller).await
}

/// Read the next floor photo through the nearest ancestor [`BlobStore`], cycling via
/// [`PhotoStream`] so successive calls see successive photos. This desktop binary's
/// capture; the browser binary serves the same `take-photo` route from the real webcam
/// in V3. `pub(crate)` so the single-binary [`InterpretPhoto`](super::InterpretPhoto)
/// can capture without a round trip until V2 moves it to the head client.
pub(crate) async fn capture(caller: &AsyncEntity) -> Result<MediaBytes> {
	// resolve the photo store (scoped to the photo dir) and the next cursor in one
	// world access, advancing the cursor so the following call sees the next photo.
	let (dir_store, cursor) = caller
		.with_state::<(AncestorQuery<&BlobStore>, ResMut<PhotoStream>), _>(
			|entity, (stores, mut photos)| -> Result<(BlobStore, usize)> {
				let dir_store =
					stores.get(entity)?.with_subdir(photos.dir.clone());
				Ok((dir_store, photos.advance()))
			},
		)
		.await??;
	read_next_photo(&dir_store, cursor).await
}

/// The floor photos the capture cycles through. The cursor advances each call and
/// wraps, so the loop keeps seeing fresh scenes.
#[derive(Debug, Clone, Resource)]
pub struct PhotoStream {
	/// The store-relative directory the photos live in.
	pub dir: SmolPath,
	/// The index of the next photo to return.
	pub cursor: usize,
}

impl Default for PhotoStream {
	fn default() -> Self {
		Self {
			dir: SmolPath::from("assets/floor-photos"),
			cursor: 0,
		}
	}
}

impl PhotoStream {
	/// Return the current cursor and advance it by one.
	fn advance(&mut self) -> usize {
		let current = self.cursor;
		self.cursor += 1;
		current
	}
}

/// List the photo dir, sort for a stable order, and read the `cursor`-th photo
/// (wrapping). Split out so the cycling is testable offline.
async fn read_next_photo(
	dir_store: &BlobStore,
	cursor: usize,
) -> Result<MediaBytes> {
	let mut paths = dir_store.list().await?;
	if paths.is_empty() {
		bevybail!("no floor photos to take");
	}
	paths.sort();
	dir_store.get_media(&paths[cursor % paths.len()]).await
}

#[cfg(test)]
mod test {
	use super::*;

	/// A temp store seeded with `count` tiny jpeg blobs under `dir`.
	async fn photo_store(dir: &str, count: usize) -> BlobStore {
		let store = BlobStore::temp();
		for index in 0..count {
			store
				.insert(
					&SmolPath::from(format!("{dir}/{index}.jpg")),
					vec![index as u8],
				)
				.await
				.unwrap();
		}
		store.with_subdir(SmolPath::from(dir))
	}

	#[beet_core::test]
	async fn cycles_and_wraps() {
		let dir_store = photo_store("photos", 3).await;
		// distinct photos in order, then wrapping back to the first.
		let first = read_next_photo(&dir_store, 0).await.unwrap();
		let second = read_next_photo(&dir_store, 1).await.unwrap();
		let wrapped = read_next_photo(&dir_store, 3).await.unwrap();
		(first.bytes() == wrapped.bytes()).xpect_true();
		(first.bytes() != second.bytes()).xpect_true();
		(first.media_type() == &MediaType::Jpeg).xpect_true();
	}

	#[beet_core::test]
	async fn empty_dir_errors() {
		read_next_photo(&photo_store("photos", 0).await, 0)
			.await
			.xpect_err();
	}
}
