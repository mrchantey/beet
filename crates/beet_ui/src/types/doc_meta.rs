//! Document metadata, declared by markdown frontmatter, a BSX root spread or
//! an Office file's core properties.
//!
//! [`DocMeta`] is inserted on a document's root entity, hoisted there by
//! whichever scan read the document (see
//! [`RootDeclarations`](beet_core::prelude::RootDeclarations)). Its
//! `title`/`description` override the
//! [`PackageConfig`](beet_core::prelude::PackageConfig) defaults in the document
//! [`Head`](crate::prelude::Head), and its `created` and `authors` are read by
//! the entry a generated index renders for it. Where the document serves and
//! how it is listed is [`RouteMeta`](crate::prelude::RouteMeta)'s.
//!
//! It lives here rather than in the router because it is DOCUMENT metadata; the
//! router is one consumer of it, as the head widgets are.
//!
//! # Metadata rules
//!
//! The limits below are not a house style, they are the smallest envelope every
//! consumer of a page's metadata accepts, so one set of frontmatter serves the
//! document head, the feed and a syndicated record with no second pass and no
//! truncation anywhere.
//!
//! - [`description`](DocMeta::description): at most 300 graphemes and 3000
//!   bytes. The tightest limit of the set, and it comes from the furthest
//!   consumer: a post's description becomes the text of the Bluesky post
//!   announcing it, and `app.bsky.feed.post.text` caps at 300 graphemes and
//!   3000 bytes. The same string is the page's meta description, its feed entry
//!   summary and its link card description. Graphemes rather than chars because
//!   an emoji with a skin-tone modifier is one thing a reader sees and should
//!   cost one unit of the budget. A dated page reserves the length of its own
//!   [date line](DocMeta::date_line) within that budget, since a post announced
//!   more than [a week](DocMeta::UNDATED_ANNOUNCE_DAYS) after its `created`
//!   day opens with one, ie `Published 11 July 2025` and a blank line, and
//!   which week a post is announced in is not known when it is written.
//! - [`title`](DocMeta::title): at most 500 graphemes and 5000 bytes, the
//!   `site.standard.document` limit, which nothing downstream undercuts.
//! - [`tags`](DocMeta::tags): each matching `^[a-z0-9]+(-[a-z0-9]+)*$` and drawn from the
//!   vocabulary its site declares. Lowercase kebab because a tag is a url
//!   segment of its own listing page and a plain string in a syndicated record,
//!   and a closed vocabulary because an open one drifts into synonyms that
//!   split one topic across two names nobody browses. At most 128 graphemes
//!   each, the `site.standard.document` tag limit; a site declaring no
//!   vocabulary checks the shape and length alone.
//!
//! [`DocMetaRules`] carries these limits and the site's vocabulary, and
//! [`DocMeta::validate`] checks a page against them; `beet check` raises each
//! [`DocMetaViolation`] as a diagnostic on the page that authored it.

#[cfg(feature = "serde")]
use crate::prelude::ReflectFrontmatter;
use beet_core::prelude::*;

/// General document metadata: a blog post's, a docs page's, a Word file's.
///
/// Flat by design: every field is a frontmatter key by the same name, so a
/// document's authored surface maps onto the component 1:1 through reflection
/// with no hand-written mapping. It is the default
/// [`FrontmatterType`](crate::prelude::FrontmatterType), so its
/// keys are written unsectioned; a route's own keys sit in a `[RouteMeta]`
/// table (see [`RouteMeta`](crate::prelude::RouteMeta)).
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Frontmatter))]
pub struct DocMeta {
	/// Document title; overrides the package title in the document `Head`.
	pub title: Option<String>,
	/// Document description; overrides the package description in the `Head`.
	pub description: Option<String>,
	/// The day the page was published.
	///
	/// Authored as a `YYYY-MM-DD` string in either surface, markdown frontmatter
	/// (`created = "2026-08-28"`) or a BSX spread (`{DocMeta{created:".."}}`),
	/// through [`Date`]'s literal parser, so it sorts and formats as a day
	/// rather than as text.
	pub created: Option<Date>,
	/// The day of the last substantive edit, the one a reader judges the
	/// page's freshness by. Authored exactly like [`created`](Self::created),
	/// and unset on a page that has not changed since publication.
	pub updated: Option<Date>,
	/// Who wrote the page, in credit order.
	///
	/// Authored as a list, ie `authors = ["Ada", "Grace"]`.
	#[cfg_attr(feature = "serde", serde(default))]
	pub authors: Vec<SmolStr>,
	/// The topics the page belongs to, lowercase kebab and drawn from the
	/// vocabulary its site declares (see the module docs' metadata rules).
	///
	/// Authored as a list, ie `tags = ["ecs", "web"]`.
	#[cfg_attr(feature = "serde", serde(default))]
	pub tags: Vec<SmolStr>,
	/// The self labels a syndicated record of the page carries, effectively
	/// content warnings, normally none.
	///
	/// Authored as a list, ie `labels = ["graphic-media"]`.
	#[cfg_attr(feature = "serde", serde(default))]
	pub labels: Vec<SmolStr>,
	/// The page's thumbnail / social card image.
	pub image_url: Option<Url>,
	/// The page's companion video, eg the YouTube watch url a post embeds.
	pub video_url: Option<Url>,
}

/// The limits and the tag vocabulary every [`DocMeta`] is checked against,
/// whose reasons are the module docs' metadata rules.
///
/// A reflected resource, so a site declares its vocabulary in markup like
/// [`PackageConfig`](beet_core::prelude::PackageConfig), ie
/// `<DocMetaRules tags={["ecs", "web"]}/>`.
#[derive(Debug, Clone, PartialEq, Eq, Resource, Reflect)]
#[reflect(Resource, Default)]
pub struct DocMetaRules {
	/// The [`title`](DocMeta::title) limit, default 500 graphemes and 5000 bytes.
	pub title: TextLimit,
	/// The [`description`](DocMeta::description) limit, default 300
	/// graphemes and 3000 bytes.
	pub description: TextLimit,
	/// The limit on each of the [`tags`](DocMeta::tags), default 128
	/// graphemes and 1280 bytes.
	pub tag: TextLimit,
	/// The vocabulary every tag is drawn from; empty declares none, so any
	/// well-formed tag passes.
	pub tags: Vec<SmolStr>,
}

impl Default for DocMetaRules {
	fn default() -> Self {
		Self {
			title: TextLimit::new(500, 5000),
			description: TextLimit::BSKY_POST_TEXT,
			tag: TextLimit::new(128, 1280),
			tags: Vec::new(),
		}
	}
}

/// A cap on a string's length in graphemes and in utf-8 bytes, the pair every
/// atproto lexicon string limit is expressed as.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Default)]
pub struct TextLimit {
	/// The most graphemes, ie the units a reader sees.
	pub graphemes: usize,
	/// The most utf-8 bytes.
	pub bytes: usize,
}

impl TextLimit {
	/// The `app.bsky.feed.post.text` limit, 300 graphemes and 3000 bytes:
	/// the whole text of a Bluesky post.
	pub const BSKY_POST_TEXT: Self = Self::new(300, 3000);

	/// A limit of `graphemes` and `bytes`.
	pub const fn new(graphemes: usize, bytes: usize) -> Self {
		Self { graphemes, bytes }
	}

	/// The [`TextLength`] of `text` when it exceeds this limit.
	pub fn exceeded_by(&self, text: &str) -> Option<TextLength> {
		let length = TextLength::of(text);
		(length.graphemes > self.graphemes || length.bytes > self.bytes)
			.then_some(length)
	}
}

/// A string's length in both units a [`TextLimit`] caps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextLength {
	/// The extended grapheme clusters.
	pub graphemes: usize,
	/// The utf-8 bytes.
	pub bytes: usize,
}

impl TextLength {
	/// The length of `text`.
	pub fn of(text: &str) -> Self {
		use unicode_segmentation::UnicodeSegmentation;
		Self {
			graphemes: text.graphemes(true).count(),
			bytes: text.len(),
		}
	}
}

/// One way a [`DocMeta`] breaks its [`DocMetaRules`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DocMetaViolation {
	/// A field longer than its limit.
	#[error(
		"`{field}` is {} graphemes and {} bytes, over the limit of {} graphemes and {} bytes",
		length.graphemes,
		length.bytes,
		limit.graphemes,
		limit.bytes
	)]
	TooLong {
		/// The frontmatter key, ie `description`, or `tags` for a single tag.
		field: &'static str,
		/// The limit it breaks.
		limit: TextLimit,
		/// Its actual length.
		length: TextLength,
	},
	/// A dated page's description within its limit alone, but over it once
	/// the [date line](DocMeta::date_line) its announcement may open with is
	/// counted.
	#[error(
		"`description` is {} graphemes and {} bytes with the date line its announcement may open with, over the limit of {} graphemes and {} bytes",
		length.graphemes,
		length.bytes,
		limit.graphemes,
		limit.bytes
	)]
	TooLongToAnnounce {
		/// The description limit it breaks.
		limit: TextLimit,
		/// The length of the date line and the description together.
		length: TextLength,
	},
	/// A tag that is not lowercase kebab.
	#[error("tag `{0}` is not lowercase kebab, ie `local-first`")]
	MalformedTag(SmolStr),
	/// A tag outside the site's vocabulary.
	#[error("tag `{0}` is not in the site's vocabulary")]
	UnknownTag(SmolStr),
}

impl DocMeta {
	/// Every way this page breaks `rules`, empty when it satisfies them.
	pub fn validate(&self, rules: &DocMetaRules) -> Vec<DocMetaViolation> {
		let too_long = |field: &'static str, limit: TextLimit, text: &str| {
			limit
				.exceeded_by(text)
				.map(|length| DocMetaViolation::TooLong {
					field,
					limit,
					length,
				})
		};
		let mut violations = Vec::new();
		violations.extend(
			self.title
				.as_deref()
				.and_then(|title| too_long("title", rules.title, title)),
		);
		violations.extend(self.description.as_deref().and_then(
			|description| {
				too_long("description", rules.description, description).or_else(
					|| {
						let dated =
							format!("{}{description}", self.date_line()?);
						rules.description.exceeded_by(&dated).map(|length| {
							DocMetaViolation::TooLongToAnnounce {
								limit: rules.description,
								length,
							}
						})
					},
				)
			},
		));
		for tag in &self.tags {
			if !Self::is_kebab(tag) {
				violations.push(DocMetaViolation::MalformedTag(tag.clone()));
			} else if !rules.tags.is_empty() && !rules.tags.contains(tag) {
				violations.push(DocMetaViolation::UnknownTag(tag.clone()));
			}
			violations.extend(too_long("tags", rules.tag, tag));
		}
		violations
	}

	/// The days after its [`created`](Self::created) day a page is announced
	/// without a [date line](Self::date_line).
	pub const UNDATED_ANNOUNCE_DAYS: i64 = 7;

	/// The line a late announcement of this page opens with, its `created`
	/// day and the blank line after it, ie `Published 11 July 2025\n\n`;
	/// `None` for an undated page.
	pub fn date_line(&self) -> Option<String> {
		self.created
			.map(|created| format!("Published {}\n\n", created.format_long()))
	}

	/// The text of the Bluesky post announcing this page on `day`, `None`
	/// without a description.
	///
	/// The description, opened by the [date line](Self::date_line) when
	/// `day` is more than [`UNDATED_ANNOUNCE_DAYS`](Self::UNDATED_ANNOUNCE_DAYS)
	/// after the page's `created`, so a back catalogue post says when it was
	/// written while the post itself is dated the day it is made, never
	/// backdated.
	pub fn announcement(&self, day: Date) -> Option<String> {
		let description = self.description.as_deref()?;
		self.created
			.filter(|created| {
				created.days_until(day) > Self::UNDATED_ANNOUNCE_DAYS
			})
			.and_then(|_| self.date_line())
			.unwrap_or_default()
			.xmap(|line| format!("{line}{description}"))
			.xsome()
	}

	/// Whether `tag` matches `^[a-z0-9]+(-[a-z0-9]+)*$`.
	fn is_kebab(tag: &str) -> bool {
		tag.split('-').all(|segment| {
			!segment.is_empty()
				&& segment.bytes().all(|byte| {
					byte.is_ascii_lowercase() || byte.is_ascii_digit()
				})
		})
	}

	/// The authors as one line, joined by `, `, or `None` when there are none:
	/// the byline a header, a feed item or a search result shows.
	pub fn byline(&self) -> Option<String> {
		(!self.authors.is_empty()).then(|| self.authors.join(", "))
	}

	/// The date a reader judges the page by: its
	/// [`updated`](Self::updated), else its [`created`](Self::created).
	pub fn last_modified(&self) -> Option<Date> {
		self.updated.or(self.created)
	}

	/// The page's social card image: its own [`image_url`](Self::image_url),
	/// else the thumbnail of the YouTube video it companions.
	///
	/// The derivation is what gets a video post a real link preview with no
	/// frontmatter at all: the poster frame IS the card. `hqdefault` rather
	/// than `maxresdefault`, which 404s for anything uploaded before high-res
	/// thumbnails and so previews as a broken image.
	pub fn social_image_url(&self) -> Option<Url> {
		self.image_url.clone().or_else(|| {
			Self::youtube_id(self.video_url.as_ref()?).map(|id| {
				Url::coerce(format!(
					"https://i.ytimg.com/vi/{id}/hqdefault.jpg"
				))
			})
		})
	}

	/// The video id in a YouTube url: the `7koepBSRoUI` in
	/// `https://youtu.be/..`, `https://www.youtube.com/watch?v=..` or an
	/// already-embed url. `None` for any other url, so a video hosted elsewhere
	/// yields neither an embed nor a derived card.
	///
	/// Read off the url's parts rather than its text, so a share link's `?t=42`
	/// and a watch link's `&list=..` are query params that were never in the
	/// path to begin with.
	pub fn youtube_id(url: &Url) -> Option<&str> {
		let host = url.host()?.trim_start_matches("www.");
		match (host, url.path().as_slice()) {
			("youtu.be", [id]) => Some(id.as_str()),
			("youtube.com", [segment, id]) if segment == "embed" => {
				Some(id.as_str())
			}
			("youtube.com", [segment]) if segment == "watch" => {
				url.get_param("v")
			}
			_ => None,
		}
	}
}

#[cfg(all(test, feature = "bsx"))]
mod test {
	use super::*;
	use crate::prelude::*;
	use bevy::reflect::TypeRegistry;

	/// Resolve a frontmatter block exactly as a scan does: lower it to root
	/// declarations, then reflect-build this one component out of them.
	fn parse(content: &str, media_type: MediaType) -> DocMeta {
		let mut registry = TypeRegistry::default();
		registry.register::<DocMeta>();
		Frontmatter::parse(content, media_type)
			.unwrap()
			.declarations(&TypeName::of::<DocMeta>().short())
			.get::<DocMeta>(&registry)
			.unwrap()
			.unwrap_or_default()
	}

	/// A `YYYY-MM-DD` string coerces to the day it names, the rule a BSX
	/// spread gets for free and frontmatter used to hand-roll.
	#[beet_core::test]
	fn frontmatter_reads_article_keys() {
		let meta = parse(
			"created = \"2025-07-11\"\nupdated = \"2025-08-01\"\nauthors = [\"Pete Hayman\", \"Ada Lovelace\"]\nvideo_url = \"https://youtu.be/7koepBSRoUI\"\ntags = [\"bevy\", \"web\"]\nlabels = [\"graphic-media\"]",
			MediaType::Toml,
		);
		meta.tags
			.xpect_eq(vec![SmolStr::new("bevy"), SmolStr::new("web")]);
		meta.labels.xpect_eq(vec![SmolStr::new("graphic-media")]);
		meta.created.unwrap().format_long().xpect_eq("11 July 2025");
		// the freshness date a sitemap and a feed read, falling back to `created`
		meta.last_modified()
			.unwrap()
			.to_string()
			.xpect_eq("2025-08-01");
		meta.authors
			.xpect_eq(vec![SmolStr::new("Pete Hayman"), "Ada Lovelace".into()]);
		meta.video_url
			.unwrap()
			.to_string()
			.xpect_eq("https://youtu.be/7koepBSRoUI");
	}

	/// The social card falls back to the companion video's poster frame, so a
	/// video post previews without any `image_url` frontmatter, and a video
	/// hosted elsewhere derives nothing.
	#[beet_core::test]
	fn derives_the_social_card_from_a_video() {
		let card = |video_url: &str| {
			DocMeta {
				video_url: Some(video_url.into()),
				..default()
			}
			.social_image_url()
		};
		let thumbnail = "https://i.ytimg.com/vi/7koepBSRoUI/hqdefault.jpg";
		card("https://youtu.be/7koepBSRoUI")
			.unwrap()
			.to_string()
			.xpect_eq(thumbnail);
		// a share url carries a timestamp, the watch url a playlist
		card("https://youtu.be/7koepBSRoUI?t=42")
			.unwrap()
			.to_string()
			.xpect_eq(thumbnail);
		card("https://www.youtube.com/watch?v=7koepBSRoUI&list=PL")
			.unwrap()
			.to_string()
			.xpect_eq(thumbnail);
		DocMeta::youtube_id(&Url::coerce(
			"https://www.youtube.com/embed/7koepBSRoUI",
		))
		.unwrap()
		.xpect_eq("7koepBSRoUI");
		card("https://example.com/video.mp4").xpect_none();
		// a bare path is no video at all
		card("/assets/clip.mp4").xpect_none();
		// an explicit image always wins over the derived one
		DocMeta {
			image_url: Some("/assets/card.png".into()),
			video_url: Some("https://youtu.be/7koepBSRoUI".into()),
			..default()
		}
		.social_image_url()
		.unwrap()
		.to_string()
		.xpect_eq("/assets/card.png");
	}

	#[beet_core::test]
	fn validates_against_rules() {
		let rules = DocMetaRules {
			tags: vec!["ecs".into(), "local-first".into()],
			..default()
		};
		DocMeta {
			title: Some("Title".into()),
			// a skin-toned emoji is one grapheme of two chars, so 300 fit the
			// description budget where a char count would refuse them
			description: Some("👍🏽".repeat(300)),
			tags: vec!["ecs".into(), "local-first".into()],
			..default()
		}
		.validate(&rules)
		.xpect_empty();
		let violations = DocMeta {
			description: Some("a".repeat(301)),
			tags: vec!["Ecs".into(), "web".into(), "a--b".into()],
			..default()
		}
		.validate(&rules);
		violations.len().xpect_eq(4);
		violations[0]
			.to_string()
			.xpect_contains("`description` is 301");
		violations[1].xpect_eq(DocMetaViolation::MalformedTag("Ecs".into()));
		violations[2].xpect_eq(DocMetaViolation::UnknownTag("web".into()));
		violations[3].xpect_eq(DocMetaViolation::MalformedTag("a--b".into()));
		// the byte cap binds where graphemes do not: a family emoji is one
		// grapheme of 25 bytes
		DocMeta {
			description: Some("👨‍👩‍👧‍👦".repeat(150)),
			..default()
		}
		.validate(&rules)
		.len()
		.xpect_eq(1);
		// no declared vocabulary checks the shape alone
		DocMeta {
			tags: vec!["anything-1".into()],
			..default()
		}
		.validate(&DocMetaRules::default())
		.xpect_empty();
	}

	/// A post announced within a week of its `created` day is its
	/// description alone; one announced later opens with its date line.
	#[beet_core::test]
	fn announces_old_posts_with_a_date_line() {
		let meta = DocMeta {
			description: Some("all about ecs".into()),
			created: Date::parse("2025-07-11").ok(),
			..default()
		};
		let day = |text: &str| Date::parse(text).unwrap();
		meta.announcement(day("2025-07-18"))
			.unwrap()
			.xpect_eq("all about ecs");
		meta.announcement(day("2025-07-19"))
			.unwrap()
			.xpect_eq("Published 11 July 2025\n\nall about ecs");
		DocMeta {
			created: None,
			..meta
		}
		.announcement(day("2026-01-01"))
		.unwrap()
		.xpect_eq("all about ecs");
	}

	/// A dated page reserves its date line within the description limit,
	/// so a description that only overflows beside it is flagged, and an
	/// undated one is not.
	#[beet_core::test]
	fn reserves_the_date_line() {
		let rules = DocMetaRules::default();
		// `Published 11 July 2025` and a blank line, 24 graphemes
		let meta = DocMeta {
			description: Some("a".repeat(277)),
			created: Date::parse("2025-07-11").ok(),
			..default()
		};
		meta.validate(&rules).xpect_eq(vec![
			DocMetaViolation::TooLongToAnnounce {
				limit: rules.description,
				length: TextLength {
					graphemes: 301,
					bytes: 301,
				},
			},
		]);
		DocMeta {
			description: Some("a".repeat(276)),
			..meta.clone()
		}
		.validate(&rules)
		.xpect_empty();
		DocMeta {
			created: None,
			..meta
		}
		.validate(&rules)
		.xpect_empty();
	}

	#[beet_core::test]
	fn defaults_when_empty() {
		let meta = parse("", MediaType::Yaml);
		meta.xpect_eq(DocMeta::default());
	}
}
