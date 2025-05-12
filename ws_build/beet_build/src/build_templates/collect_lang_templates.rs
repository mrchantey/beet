use super::ExtractedLangContent;
use super::ExtractedLangTemplate;
use super::error::Error;
use super::error::Result;
use beet_common::prelude::*;
use beet_rsx::prelude::*;
use rapidhash::RapidHashMap;
use sweet::prelude::*;

/// Once templates are extracted, we need to collect them into a single
/// hash map, deduplicating identical templates, loading files etc.
pub struct CollectLangTemplates;



impl Pipeline<Vec<ExtractedLangTemplate>, Result<LangTemplateMap>>
	for CollectLangTemplates
{
	fn apply(
		self,
		templates: Vec<ExtractedLangTemplate>,
	) -> Result<LangTemplateMap> {
		let mut template_map: RapidHashMap<LangContentHash, LangTemplate> =
			RapidHashMap::default();
		let mut id = 0;
		for ExtractedLangTemplate {
			span,
			content,
			tag,
			directives,
			content_hash,
		} in templates.into_iter()
		{
			if let Some(entry) = template_map.get_mut(&content_hash) {
				// if the template already exists, just add the span
				entry.push_span(span);
				continue;
			}

			let content = match content {
				ExtractedLangContent::File(path) => {
					let path = path
						.into_abs()
						.map_err(|e| Error::collect_lang_templates(&span, e))?;
					ReadFile::to_string(&path)
						.map_err(|e| Error::collect_lang_templates(&span, e))?
				}
				ExtractedLangContent::Inline(content) => content,
			};

			let template = LangTemplate::new(
				tag,
				id,
				directives,
				content,
				content_hash,
				vec![span],
			);
			id += 1;

			template_map.insert(content_hash, template);
		}
		Ok(LangTemplateMap::new(template_map))
	}
}

#[cfg(test)]
mod test {
	use crate::build_templates::*;
	// use beet_common::prelude::*;
	use crate::as_beet::*;
	use anyhow::Result;
	use sweet::prelude::*;

	fn parse(tokens: WebTokens) -> Result<LangTemplateMap> {
		tokens
			.xpipe(ExtractLangTemplates::default())?
			.1
			.xpipe(CollectLangTemplates)?
			.xok()
	}

	#[test]
	fn works() {
		web_tokens! {
			<style> body{} </style>
			// same template
			<style> body{} </style>
			// same inner, different tag
			<script> body{} </script>
			// same inner, different variant
			<style src="./test.css"/>
			// same inner, different directive
			<script scope:global> body{} </script>
		}
		.xmap(parse)
		.unwrap()
		.xmap(|map| map.len())
		.xpect()
		.to_be(4);
	}

	#[test]
	fn errors() {
		// relative ignored
		web_tokens! { <script src="/missing" /> }
			.xmap(parse)
			.unwrap()
			.xmap(|map| map.len())
			.xpect()
			.to_be(0);

		expect(web_tokens! { <script src="./missing" /> }.xmap(parse))
			.to_be_err();
		// slot children errors
		expect(
			web_tokens! {
				<Foo>
					<script src="./missing" />
				</Foo>
			}
			.xmap(parse),
		)
		.to_be_err();
	}
}
