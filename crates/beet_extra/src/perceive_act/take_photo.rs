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

/// Capture from the mock camera: read the next floor photo from the nearest
/// self-or-ancestor [`BlobStore`], cycling via [`PhotoStream`] so successive calls
/// see successive photos. The store is mounted in markup at the floor-photo dir, eg
/// `<TakePhoto {FsStore{path:"assets/floor-photos"}}/>`. The v1 stand-in for the
/// real webcam.
async fn mock_capture(caller: &AsyncEntity) -> Result<MediaBytes> {
	// resolve the photo store and the next cursor in one world access, advancing the
	// cursor so the following call sees the next photo.
	let (store, cursor) = caller
		.with_state::<(AncestorQuery<&BlobStore>, ResMut<PhotoStream>), _>(
			|entity, (stores, mut photos)| -> Result<(BlobStore, usize)> {
				Ok((stores.get(entity)?.clone(), photos.advance()))
			},
		)
		.await??;
	read_next_photo(&store, cursor).await
}

/// The mock camera's cursor over its photo store: it advances each call and wraps,
/// so the loop keeps seeing fresh scenes. The photos themselves come from the
/// nearest self-or-ancestor [`BlobStore`] (see [`mock_capture`]).
#[derive(Debug, Default, Clone, Resource)]
pub struct PhotoStream {
	/// The index of the next photo to return.
	pub cursor: usize,
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

	/// The mock camera resolves its photos from the nearest self-or-ancestor
	/// [`BlobStore`] (the markup-mounted `{FsStore}`) and advances the shared cursor
	/// across calls, the offline half of [`mock_capture`].
	#[beet_core::test]
	async fn resolves_photos_from_ancestor_store() {
		let mut world = World::new();
		world.init_resource::<PhotoStream>();
		// store on an ancestor, camera reads it via self-or-ancestor lookup.
		let parent = world.spawn(photo_store("photos", 3).await).id();
		let camera = world.spawn(ChildOf(parent)).id();
		world.flush();
		// resolve the store + advance the cursor exactly as `mock_capture` does.
		let resolve = |world: &mut World| {
			world.with_state::<(AncestorQuery<&BlobStore>, ResMut<PhotoStream>), _>(
				|(stores, mut photos)| -> Result<(BlobStore, usize)> {
					Ok((stores.get(camera)?.clone(), photos.advance()))
				},
			)
		};
		let (store, cursor) = resolve(&mut world).unwrap();
		let first = read_next_photo(&store, cursor).await.unwrap();
		let (store, cursor) = resolve(&mut world).unwrap();
		let second = read_next_photo(&store, cursor).await.unwrap();
		// the cursor advanced, so successive calls see successive photos.
		cursor.xpect_eq(1);
		(first.bytes() != second.bytes()).xpect_true();
	}
}
