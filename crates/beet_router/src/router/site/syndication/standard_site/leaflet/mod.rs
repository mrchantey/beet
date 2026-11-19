//! Leaflet's block model, `pub.leaflet.content`, an optional extra under the
//! `standard_site_extra` feature: the format Leaflet's own reader, and the
//! third party standard.site readers built against it, read. It is never a
//! publication's default and no deployed binary carries it; it is here for
//! anyone reading Leaflet documents into beet, or writing their own posts in
//! Leaflet's format, a publication naming `content="pub.leaflet.content"`.
//!
//! - [`LeafletContent`] and its blocks, the lexicon types, each serialized
//!   with Leaflet's exact wire names
//! - [`LeafletRenderer`], a [`LexiconRenderer`] of `pub.leaflet.content`
//! - [`LeafletParser`], a [`LexiconParser`] per def, reading a Leaflet document
//!   into the elements the markdown parser produces
//!
//! Both are installed by `RouterPlugin` with the feature,
//! `app.install((LeafletRenderer, LeafletParser::all()))`.
//!
//! The format is a small block model, a subset of html. A beet post is a
//! markdown document whose parsed article is a superset of it, so the
//! conversion is lossy by design and the canonical page stays the full post.
//!
//! # The shape
//!
//! One `pages` entry per document, a `pub.leaflet.pages.linearDocument` holding
//! an array of `{block: <union member>}`. No canvas page: a post is linear.
//!
//! # What maps
//!
//! | parsed article node | block |
//! | --- | --- |
//! | `h1`..`h6` | `blocks.header`, `level` from the tag |
//! | `p` | `blocks.text` |
//! | `blockquote` | `blocks.blockquote`, its paragraphs joined by a blank line |
//! | `pre > code` | `blocks.code`, `language` from the code's class (`rust`, or `language-rust`) |
//! | `hr` | `blocks.horizontalRule` |
//! | `ul`, `ol` | `blocks.unorderedList`, `blocks.orderedList`; each item's `content` is a `blocks.text`, and a nested list rides `children` |
//! | `img` | `blocks.image`: the `BlobRef` of the `InlineBlob` the media resolve step left on its `src` attribute, and the `aspectRatio` the lexicon requires read from the bytes' header; an image not fetched (a `link` policy, or a request naming none) is a `blocks.website` with its url |
//! | `a`, `strong`/`b`, `em`/`i`, `code` (inline), `mark`, `u`, `s`, `del` | not blocks: facets over the enclosing block's `plaintext` |
//!
//! # Facets
//!
//! A facet is a byte range over the block's `plaintext` plus a list of features,
//! so inline markup is a side table rather than a tree. Byte offsets into the
//! UTF-8 encoding, not chars and not graphemes. Features used: `#link`, `#bold`,
//! `#italic`, `#code`, `#highlight`, `#underline`, `#strikethrough`. Nesting is
//! expressed as two facets over overlapping ranges, which is the only way the
//! model can say it. Unused: `#didMention`, `#atMention`, `#id`, `#footnote`.
//!
//! # Links
//!
//! A record is read far from the site, so every link is absolute: a relative
//! `href` or `src` resolves against the page's url, the site's homepage
//! ([`PackageConfig::absolute_url`]) over the path of the request the render
//! answers. A relative link with no homepage declared is an error naming
//! `PackageConfig.homepage`, as the sitemap and the feeds refuse one.
//!
//! # What degrades, and to what
//!
//! Every row here is a deliberate choice of the most legible lossy form, never a
//! silent drop:
//!
//! | node | degrade |
//! | --- | --- |
//! | `table` | a `blocks.code` with no language, holding the pipe table. Monospace keeps the columns aligned and a reader can see it is a table, where prose cannot say it at all |
//! | a mermaid fence | a `blocks.code` with `language: "mermaid"`, which is what the source said |
//! | `iframe` (the YouTube embed every video post carries) | a `blocks.website` with the watch url, ie a link card every reader can render, rather than `blocks.iframe`, which only Leaflet's own reader honours. The canonical page keeps the real embed |
//! | a client island or any other unregistered element | dropped, with its text content emitted as a `blocks.text` when it has any |
//!
//! Nothing beet authors maps to `blocks.math`, `blocks.poll`, `blocks.button`,
//! `blocks.drawing`, `blocks.embeddedCanvas`, `blocks.signup`,
//! `blocks.postsList`, `blocks.recommendedPubs`, `blocks.postHeader` or
//! `blocks.membersOnlyDelimiter`, so none is implemented.
//!
//! # The size ceiling
//!
//! A repo block is capped at a megabyte and a reader fetches a record whole, so
//! a document has a budget, checked once where it is rendered whatever its
//! `content` (`StandardSiteDocument::BUDGET`). The lexicon's escape hatch is
//! `blobPages`, an `application/json` blob holding the page array, and when it
//! is set every blob referenced inside it MUST be mirrored in the record's top
//! level `blobs` array, because a PDS scans only the top level of a current
//! record when deciding what to garbage collect.
//!
//! Beet does not write `blobPages`. The longest post in the corpus is about a
//! thousand words, which is single digit kilobytes of blocks, and images are
//! blobs rather than inline bytes, so a post over the budget fails loudly at
//! the document and `blobPages` is the fix to build then. An unused code path
//! that only a future post would exercise is a path nothing tests.
//!
//! # Reading a Leaflet document
//!
//! Each def parses onto its own entity, a page's blocks and a list's items
//! children in order, so the html renderer renders a Leaflet post knowing
//! nothing of Leaflet:
//!
//! | def | elements |
//! | --- | --- |
//! | `pub.leaflet.content`, `pages.linearDocument` | no element, its pages and their blocks as children |
//! | `blocks.text` | `p`, empty when its text is (Leaflet's spacer) |
//! | `blocks.header` | `h1`..`h6` from `level` |
//! | `blocks.blockquote` | `blockquote`, a `p` per blank-line-separated paragraph |
//! | `blocks.code` | `pre > code`, `class` the bare language as a markdown fence writes it |
//! | `blocks.horizontalRule` | `hr` |
//! | `blocks.unorderedList`, `blocks.orderedList` | `ul`, `ol` with its `start`; each `#listItem` an `li` holding its text inline, as a tight list does, then its nested lists |
//! | `blocks.image` | `img`, its `src` the blob on the repo the record was read from ([`LexiconSource::blob_url`]); read with no source, no `src`, warned once per document |
//! | `blocks.website` | `p > a`, its text the title, else the url |
//! | `blocks.iframe` | `iframe` |
//!
//! Facets become the inline elements the renderer reads them from (`a`,
//! `strong`, `em`, `code`, `mark`, `u`, `del`), nested where their ranges
//! nest and split where they cross, and a newline in a block's text is a `br`.
//! A feature with no element (`#didMention`, `#atMention`, `#footnote`, `#id`)
//! keeps its text unmarked.
//!
//! Any other block (`blocks.button`, `blocks.bskyPost`, `blocks.poll`, ..)
//! lands whole as an [`UnknownLexicon`], which [`LeafletRenderer`] writes back
//! verbatim in its slot, so reading a document and rendering it again keeps
//! every block beet does not know.
//!
//! # The companion video
//!
//! A post's companion video is its `DocMeta::video_url`, which `ArticleHeader`
//! renders and a `--root=content` render therefore leaves out. The renderer
//! reads it off the `DocMeta` on the render root, as the cover image reads
//! `social_image_url`, and leads the document with it: a `blocks.iframe` of
//! the video's embed url at 16:9 for a YouTube video, the block Leaflet's own
//! records carry for one, else a `blocks.website` with the url.
//!
//! [`LexiconRenderer`]: beet_ui::prelude::LexiconRenderer
//! [`LexiconParser`]: beet_ui::prelude::LexiconParser
//! [`LexiconSource::blob_url`]: beet_ui::prelude::LexiconSource::blob_url
//! [`UnknownLexicon`]: beet_ui::prelude::UnknownLexicon
mod leaflet_content;
mod leaflet_parser;
mod leaflet_renderer;
pub use leaflet_content::*;
pub use leaflet_parser::*;
pub use leaflet_renderer::*;
