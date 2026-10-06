//! Post text and the facets that make its links, mentions and hashtags live.
use crate::prelude::*;
use beet_core::prelude::*;

/// A post's plain text with its [`Facet`]s computed: a link, a mention or a
/// hashtag in post text is only live when a facet names its byte range, so a
/// writer computes them rather than refusing them.
///
/// Detected at the start of the text or after whitespace (a link or mention
/// also after `(`), with trailing punctuation left out of the range:
///
/// - a link is an explicit `http://` or `https://` url;
/// - a mention is `@` and a handle with at least one dot, resolved to its did
///   through the caller's resolver, so no AppView is needed; one that does not
///   resolve stays plain text, as it does in the app;
/// - a hashtag is `#` and up to 64 characters that are not all digits.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # async_ext::block_on(async {
/// let rich = RichText::new("Bevy ECS as the foundation, see https://beet.org", async |_| {
/// 	bevybail!("no mentions here")
/// })
/// .await?;
/// rich.facets[0].index.byte_start.xpect_eq(32);
/// rich.facets[0].index.byte_end.xpect_eq(48);
/// # Ok::<_, BevyError>(())
/// # })
/// # .unwrap();
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RichText {
	/// The plain text.
	pub text: String,
	/// Its live ranges, ordered by start.
	pub facets: Vec<Facet>,
}

impl RichText {
	/// The longest hashtag the app makes live, in characters.
	pub const MAX_TAG_CHARS: usize = 64;

	/// `text` with its facets, each mentioned handle resolved through
	/// `resolve`.
	pub async fn new(
		text: impl Into<String>,
		resolve: impl AsyncFn(&str) -> Result<Did>,
	) -> Result<Self> {
		let text = text.into();
		let mut facets = Vec::new();
		for (index, candidate) in Self::detect(&text) {
			let feature = match candidate {
				Candidate::Link(uri) => FacetFeature::Link { uri },
				Candidate::Tag(tag) => FacetFeature::Tag { tag },
				Candidate::Mention(handle) => match resolve(&handle).await {
					Ok(did) => FacetFeature::Mention { did },
					Err(err) => {
						warn!("@{handle} stays plain text: {err}");
						continue;
					}
				},
			};
			facets.push(Facet {
				index,
				features: vec![feature],
			});
		}
		Self { text, facets }.xok()
	}

	/// `text` with its facets, mentions resolved through `resolver`.
	pub async fn resolve(
		text: impl Into<String>,
		resolver: &HandleResolver,
	) -> Result<Self> {
		Self::new(text, async |handle| resolver.resolve(handle).await).await
	}

	/// Every live range of `text`, ordered by start.
	fn detect(text: &str) -> Vec<(ByteSlice, Candidate)> {
		let mut found = Vec::new();
		let mut previous: Option<char> = None;
		for (start, char) in text.char_indices() {
			let after_space = previous.is_none_or(char::is_whitespace);
			let after_paren = after_space || previous == Some('(');
			previous = Some(char);
			let rest = &text[start..];
			let word =
				&rest[..rest.find(char::is_whitespace).unwrap_or(rest.len())];
			let candidate = match char {
				'h' if after_paren
					&& (word.starts_with("https://")
						|| word.starts_with("http://")) =>
				{
					Self::link(word)
				}
				'@' if after_paren => Self::mention(&word[1..]),
				'#' | '＃' if after_space => {
					Self::tag(&word[char.len_utf8()..])
						.map(|(len, tag)| (len + char.len_utf8(), tag))
				}
				_ => None,
			};
			if let Some((len, candidate)) = candidate {
				found.push((
					ByteSlice {
						byte_start: start,
						byte_end: start + len,
					},
					candidate,
				));
			}
		}
		found
	}

	/// The url at the start of `word`, its trailing punctuation and an
	/// unbalanced closing paren left out.
	fn link(word: &str) -> Option<(usize, Candidate)> {
		let mut url =
			word.trim_end_matches(['.', ',', ';', ':', '!', '?', '"', '\'']);
		if url.ends_with(')') && !url.contains('(') {
			url = &url[..url.len() - 1];
		}
		(url.len() > "https://".len())
			.then(|| (url.len(), Candidate::Link(url.into())))
	}

	/// The handle at the start of `word`, `@` excluded, its length counting
	/// the `@`.
	fn mention(word: &str) -> Option<(usize, Candidate)> {
		let handle = &word[..word
			.find(|char: char| {
				!(char.is_ascii_alphanumeric() || char == '.' || char == '-')
			})
			.unwrap_or(word.len())];
		let handle = handle.trim_end_matches(['.', '-']);
		let labels = handle.split('.').collect::<Vec<_>>();
		(labels.len() >= 2
			&& labels.iter().all(|label| !label.is_empty())
			&& labels.last().is_some_and(|tld| {
				tld.starts_with(|c: char| c.is_ascii_alphabetic())
			}))
		.then(|| (handle.len() + 1, Candidate::Mention(handle.into())))
	}

	/// The tag at the start of `word`, `#` excluded, with its byte length.
	fn tag(word: &str) -> Option<(usize, Candidate)> {
		let tag =
			word.trim_end_matches(|char: char| char.is_ascii_punctuation());
		(!tag.is_empty()
			&& tag.chars().count() <= Self::MAX_TAG_CHARS
			&& !tag.chars().all(|char| char.is_ascii_digit()))
		.then(|| (tag.len(), Candidate::Tag(tag.into())))
	}
}

/// A range [`RichText`] found, before any mention is resolved.
enum Candidate {
	Link(SmolStr),
	Mention(SmolStr),
	Tag(SmolStr),
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// `text`'s facets, every mention resolving to the test did except
	/// `@missing.example`.
	async fn facets(text: &str) -> Vec<(usize, usize, FacetFeature)> {
		RichText::new(text, async |handle| match handle {
			"missing.example" => bevybail!("no such handle"),
			_ => Did::parse(EmulatorPds::TEST_DID),
		})
		.await
		.unwrap()
		.facets
		.into_iter()
		.map(|facet| {
			(
				facet.index.byte_start,
				facet.index.byte_end,
				facet.features[0].clone(),
			)
		})
		.collect()
	}

	/// Offsets are bytes of the UTF-8 text, so text after an emoji or an
	/// accented letter is offset by its encoded width, not its char count.
	#[beet_core::test]
	async fn offsets_are_bytes() {
		let text = "🤘 café https://beet.org, #bevy @pete.beet.org.";
		let found = facets(text).await;
		found.len().xpect_eq(3);
		let slice = |(start, end, _): &(usize, usize, FacetFeature)| {
			text[*start..*end].to_string()
		};
		slice(&found[0]).xpect_eq("https://beet.org");
		slice(&found[1]).xpect_eq("#bevy");
		slice(&found[2]).xpect_eq("@pete.beet.org");
		found[0].0.xpect_eq(11);
		found[1]
			.2
			.clone()
			.xpect_eq(FacetFeature::Tag { tag: "bevy".into() });
		found[2].2.clone().xpect_eq(FacetFeature::Mention {
			did: Did::parse(EmulatorPds::TEST_DID).unwrap(),
		});
	}

	#[beet_core::test]
	async fn ignores_what_is_not_live() {
		facets(
			"email me@example.com, issue #123, a#b, @nodot, @missing.example",
		)
		.await
		.xpect_eq(vec![]);
		facets("(see https://beet.org/docs)")
			.await
			.into_iter()
			.map(|(_, _, feature)| feature)
			.collect::<Vec<_>>()
			.xpect_eq(vec![FacetFeature::Link {
				uri: "https://beet.org/docs".into(),
			}]);
	}

	/// A facet serializes as the lexicon writes it.
	#[beet_core::test]
	async fn serializes_as_the_lexicon() {
		let rich =
			RichText::new("see https://beet.org", async |_| bevybail!("none"))
				.await
				.unwrap();
		serde_json::to_string(&rich.facets)
			.unwrap()
			.xpect_eq(r#"[{"index":{"byteStart":4,"byteEnd":20},"features":[{"$type":"app.bsky.richtext.facet#link","uri":"https://beet.org"}]}]"#);
	}
}
