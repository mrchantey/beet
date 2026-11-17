//! The `app.bsky.*` lexicons beet reads or writes, one file per lexicon, each
//! type named for the def it implements (`app.bsky.feed.post` is [`FeedPost`],
//! its `#replyRef` is [`ReplyRef`]) and serialized with the lexicon's exact
//! wire names, so a type is its lexicon and nothing else.
//!
//! Implemented as beet needs them rather than wholesale: a field beet neither
//! reads nor writes is left to the record's own serde defaults. The views an
//! AppView hydrates (`app.bsky.*.defs`) are read through [`AppView`].
mod actor_defs;
#[cfg(feature = "atproto")]
mod app_view;
mod embed_external;
mod feed_defs;
mod feed_post;
mod feed_repost;
mod rich_text;
mod richtext_facet;
pub use actor_defs::*;
#[cfg(feature = "atproto")]
pub use app_view::*;
pub use embed_external::*;
pub use feed_defs::*;
pub use feed_post::*;
pub use feed_repost::*;
pub use rich_text::*;
pub use richtext_facet::*;
