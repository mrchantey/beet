//! The `app.bsky.*` lexicons beet reads or writes, one file per lexicon, each
//! type named for the def it implements (`app.bsky.feed.post` is [`FeedPost`],
//! its `#replyRef` is [`ReplyRef`]) and serialized with the lexicon's exact
//! wire names, so a type is its lexicon and nothing else.
//!
//! Implemented as beet needs them rather than wholesale: a field beet neither
//! reads nor writes is left to the record's own serde defaults.
mod feed_post;
mod rich_text;
mod richtext_facet;
pub use feed_post::*;
pub use rich_text::*;
pub use richtext_facet::*;
