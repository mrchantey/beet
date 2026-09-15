//! [`Loading`]: the mark on an entity whose content is still arriving from a
//! store, the one signal a surface waits on before its first paint.
use beet_core::prelude::*;

/// Marks an entity whose content is still arriving from a store: a scene
/// blob, a document blob or a document naming a located schema, from its
/// spawn until its read lands.
///
/// One mark for every reader, so a surface answers "has this page settled"
/// with one walk. A store reader carries it from spawn (`#[require(Loading)]`),
/// so a tree is unsettled the frame its readers appear rather than the frame
/// their reads begin, and nothing paints a page whose scene has not arrived.
/// It doubles as the readers' in-flight guard: a store churning while a read
/// runs issues no second one.
///
/// Cleared when the read lands or fails and is reported ([`Loading::clear`]),
/// since the world is then as settled as it is going to be and the next change
/// retries the read. A blob reader's read only runs once its [`Blob`] has
/// resolved, so it stays [`Pending`](Loading::Pending) until a store arrives
/// above it; a document naming a located schema resolves its store itself and
/// stays pending when none has arrived ([`Loading::settle`]).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component)]
pub enum Loading {
	/// Content is expected and no read is running: the reader was just
	/// spawned, or no ancestor store has arrived to read from.
	#[default]
	Pending,
	/// A read is in flight.
	InFlight,
}

impl Loading {
	/// Whether a reader may issue a read: nothing is already in flight.
	pub fn may_read(loading: Option<&Loading>) -> bool {
		!matches!(loading, Some(Loading::InFlight))
	}

	/// Clear the mark on `entity`: its read landed, or failed and was reported
	/// by the caller, retried on the next change.
	#[cfg(feature = "json")]
	pub(crate) async fn clear(entity: &AsyncEntity) -> Result {
		entity
			.with(|mut entity| {
				entity.remove::<Loading>();
			})
			.await
	}

	/// Record a store-resolving read's outcome on `entity`: landed or failed
	/// clears the mark ([`clear`](Self::clear)); no store yet leaves the read
	/// pending.
	#[cfg(feature = "json")]
	pub(crate) async fn settle(
		entity: &AsyncEntity,
		outcome: &Result<Read>,
	) -> Result {
		match outcome {
			Ok(Read::NoStore) => {
				entity
					.with(|mut entity| {
						entity.insert(Loading::Pending);
					})
					.await
			}
			_ => Self::clear(entity).await,
		}
	}
}

/// What a store read came to, as [`Loading::settle`] records it.
#[cfg(feature = "json")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Read {
	/// The content landed on its entity.
	Landed,
	/// No ancestor store has arrived; the read is retried when one does.
	NoStore,
}
