use crate::prelude::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::LazyLock;
use sweet::prelude::GlobFilter;


pub struct KnownSources;

impl std::ops::Deref for KnownSources {
	type Target = HashMap<ContentSourceKey, ContentSource>;
	fn deref(&self) -> &Self::Target { &*KNOWN_SOURCES }
}

impl KnownSources {
	pub fn get(key: &ContentSourceKey) -> Result<&ContentSource> {
		KNOWN_SOURCES.get(key).ok_or_else(|| {
			anyhow::anyhow!("Could not find source for crate: {}", key)
		})
	}
	pub fn assert_exists(key: &ContentSourceKey) -> Result<()> {
		if KNOWN_SOURCES.contains_key(key) {
			Ok(())
		} else {
			Err(anyhow::anyhow!("Could not find source for crate: {}", key))
		}
	}
}


struct KnownSourceBuilder {
	crate_name: String,
	/// usually sources share a single repo url for each content type.
	content_types: HashMap<ContentType, ContentTypeBuilder>,
	sources: Vec<SourceBuilder>,
}


impl KnownSourceBuilder {
	pub fn new(crate_name: &str) -> Self {
		Self {
			crate_name: crate_name.to_string(),
			content_types: HashMap::new(),
			sources: Vec::new(),
		}
	}
	pub fn add_content_type(
		mut self,
		content_type: ContentType,
		git_url: &str,
	) -> Self {
		self.content_types.insert(
			content_type,
			ContentTypeBuilder::default_for_content_type(git_url, content_type),
		);
		self
	}
	pub fn add_source(
		self,
		content_types: &[ContentType],
		version: &str,
		git_hash: &str,
	) -> Self {
		self.add_source_with_branch(content_types, version, git_hash, "main")
	}
	pub fn add_source_with_branch(
		mut self,
		content_types: &[ContentType],
		version: &str,
		git_hash: &str,
		git_branch: &str,
	) -> Self {
		for content_type in content_types {
			self.sources.push(SourceBuilder {
				content_type: content_type.clone(),
				git_hash: git_hash.to_string(),
				git_branch: git_branch.to_string(),
				version: version.to_string(),
			});
		}
		self
	}

	pub fn build(self) -> HashMap<ContentSourceKey, ContentSource> {
		self.sources
			.into_iter()
			.map(|source| {
				let content_type_builder = self
					.content_types
					.get(&source.content_type)
					.expect(&format!(
						"Content type {} not found for crate {}",
						source.content_type, self.crate_name
					));
				let key = ContentSourceKey {
					crate_meta: CrateMeta {
						crate_name: self.crate_name.clone(),
						crate_version: source.version,
					},
					content_type: source.content_type,
				};
				let content_source = ContentSource {
					crate_meta: key.crate_meta.clone(),
					filter: content_type_builder.filter.clone(),
					git_url: content_type_builder.git_url.clone(),
					git_hash: source.git_hash,
					git_branch: source.git_branch,
					split_text: content_type_builder.split_text.clone(),
				};
				(key, content_source)
			})
			.collect()
	}
}

struct ContentTypeBuilder {
	git_url: String,
	split_text: SplitText,
	filter: GlobFilter,
}

impl ContentTypeBuilder {
	/// sensible defaults for rust repositories.
	pub fn default_for_content_type(
		git_url: &str,
		content_type: ContentType,
	) -> Self {
		let filter = GlobFilter::default().with_exclude("*.git*");
		match content_type {
			ContentType::Docs => Self {
				git_url: git_url.to_string(),
				split_text: SplitText::default(),
				filter: filter.with_include("**/*.md"),
			},
			ContentType::Examples => Self {
				git_url: git_url.to_string(),
				split_text: SplitText::default(),
				filter: filter.with_include("*examples/**/*.rs"),
			},
			ContentType::Guides => Self {
				git_url: git_url.to_string(),
				split_text: SplitText::default(),
				filter: filter.with_include("**/*.md"),
			},
			ContentType::Internals => Self {
				git_url: git_url.to_string(),
				split_text: SplitText::default(),
				filter: filter.with_include("*src/**/*.rs"),
			},
		}
	}
}

struct SourceBuilder {
	content_type: ContentType,
	git_hash: String,
	git_branch: String,
	version: String,
}

static KNOWN_SOURCES: LazyLock<HashMap<ContentSourceKey, ContentSource>> =
	LazyLock::new(|| {
		let mut map = HashMap::new();

		map.extend(
			KnownSourceBuilder::new("bevy")
				.add_content_type(
					ContentType::Examples,
					"https://github.com/BevyEngine/bevy.git",
				)
				.add_content_type(
					ContentType::Internals,
					"https://github.com/BevyEngine/bevy.git",
				)
				.add_content_type(
					ContentType::Guides,
					"https://github.com/bevyengine/bevy-website.git",
				)
				.add_source(
					&[ContentType::Examples, ContentType::Internals],
					"0.4.0",
					"0149c4145f0f398e9fba85c2584d0481a260f57c",
				)
				.add_source(
					&[ContentType::Examples, ContentType::Internals],
					"0.8.0",
					"0149c4145f0f398e9fba85c2584d0481a260f57c",
				)
				.add_source(
					&[ContentType::Examples, ContentType::Internals],
					"0.16.0",
					"e9418b3845c1ffc9624a3a4003bde66a2ad6566a",
				)
				// guides should be the last commit before the next release
				// to capture as much content as possible.
				.add_source(
					&[ContentType::Guides],
					"0.16.0",
					"166e7d46b2768b905ee71783ae9a0ea609761a36", // latest
				)
				.add_source(
					&[ContentType::Guides],
					"0.8.0",
					"5fcc38e6b38c16d02eb05c112f08d95b32ff9654", // just before 0.9
				)
				.add_source(
					&[ContentType::Guides],
					"0.4.0",
					"714ad927bd53e3db903a2adaaed531a9dbd5c7f3", // just before 0.5
				)
				.build(),
		);

		map
	});



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		expect(KnownSources::get(&ContentSourceKey::new(
			"bevy",
			"0.16.0",
			ContentType::Examples,
		)))
		.to_be_ok();
	}
}
