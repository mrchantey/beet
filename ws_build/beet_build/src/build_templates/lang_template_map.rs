use crate::prelude::*;
use beet_common::prelude::*;
use rapidhash::RapidHashMap;
use serde::Deserialize;
use serde::Serialize;



#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LangTemplateMap {
	/// Map of all templates where the key is the hash of the template.
	pub templates: RapidHashMap<u64, LangTemplateInstance>,
}

impl std::ops::Deref for LangTemplateMap {
	type Target = RapidHashMap<u64, LangTemplateInstance>;
	fn deref(&self) -> &Self::Target { &self.templates }
}


impl LangTemplateMap {
	pub fn new(templates: Vec<(FileSpan, LangTemplate)>) -> Self {
		let mut map = Self {
			templates: RapidHashMap::default(),
		};
		let mut id = 0;
		for (span, template) in templates {
			let hash = template.hash_self();
			map.templates
				.entry(hash)
				.or_insert_with(|| {
					let template = LangTemplateInstance {
						template: template.clone(),
						spans: Vec::new(),
						id,
					};
					id += 1;
					template
				})
				.spans
				.push(span);
		}
		map
	}
}


/// A template and accompanying metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LangTemplateInstance {
	/// The template itself, including the element content and
	/// any directives.
	template: LangTemplate,
	/// Each span that references this template,
	/// there may be multiple because we deduplicate identical
	/// templates.
	spans: Vec<FileSpan>,
	/// A unique id for this template, counted up from 0 so suitable
	/// as a short html data attribute.
	id: u64,
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_common::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let map = LangTemplateMap::new(vec![
			(FileSpan::default(), LangTemplate {
				directives: vec![],
				content: LangContent::Inline("bar".to_string()),
			}),
			// same template
			(FileSpan::default(), LangTemplate {
				directives: vec![],
				content: LangContent::Inline("bar".to_string()),
			}),
			// same inner, different variant
			(FileSpan::default(), LangTemplate {
				directives: vec![],
				content: LangContent::File("bar".into()),
			}),
			// same inner, different directive
			(FileSpan::default(), LangTemplate {
				directives: vec![TemplateDirective::StyleScope(
					StyleScope::Global,
				)],
				content: LangContent::File("bar".into()),
			}),
		]);
		expect(map.templates.len()).to_be(3);
		expect(map.templates.values().next().unwrap().spans.len()).to_be(2);
	}
}
