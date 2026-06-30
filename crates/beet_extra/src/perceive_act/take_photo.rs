//! `TakePhoto`: capture what is in front of the robot and have a vision model
//! describe it. v1 ships only the mock camera (the floor-photo fixtures); the real
//! camera (the browser webcam) lands in v3.
use crate::beet::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Take a photo of what is in front of you and get back a description of it.
///
/// Captures an image (the mock camera reads the floor-photo fixtures; the real
/// camera is not yet wired), one-shots it to a vision model, and returns the
/// description.
#[action(route = "take-photo")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn TakePhoto(cx: ActionContext<TakePhotoInput>) -> Result<String> {
	let media = if cx.input.mock {
		mock_capture(&cx.caller).await?
	} else {
		// the real camera (browser getUserMedia) lands in v3.
		unimplemented!(
			"real camera capture is not wired yet; call take-photo with mock = true"
		)
	};
	describe_image(media).await
}

/// How to capture the photo.
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
pub struct TakePhotoInput {
	/// Use the mock camera (the floor-photo fixtures). The real camera is not yet
	/// available, so set this `true` for now.
	pub mock: bool,
}

// --- mock camera (v1) ---

/// Capture from the mock camera: read the next floor photo through the nearest
/// ancestor [`BlobStore`], cycling via [`PhotoStream`] so successive calls see
/// successive photos. The v1 stand-in for the real webcam.
async fn mock_capture(caller: &AsyncEntity) -> Result<MediaBytes> {
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

/// The floor photos the mock camera cycles through. The cursor advances each call
/// and wraps, so the loop keeps seeing fresh scenes.
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

// --- vision (shared by every camera) ---

/// One-shot the captured photo to a vision model and return its description. Swap the
/// streamer line to change provider (the agent itself is set in the scene's
/// `{ModelStreamer}`).
async fn describe_image(media: MediaBytes) -> Result<String> {
	run_oneshot(children![
		(
			Actor::user(),
			children![
				Post::spawn(
					"You are the eyes of a small floor robot. In one or two sentences, \
					describe anything of interest in front of you, and any obstacle worth avoiding."
				),
				Post::spawn(IntoPost::Bytes {
					media_type: media.media_type().clone(),
					bytes: media.bytes().to_vec(),
					file_stem: None,
				}),
			]
		),
		(Actor::agent(), OpenAiProvider::gpt_5_mini()?),
	])
	.await?
	.into_iter()
	.filter(|post| post.intent().is_display())
	.map(|post| post.to_string())
	.collect::<String>()
	.xok()
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
