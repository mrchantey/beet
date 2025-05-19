use crate::prelude::*;
use beet_common::prelude::*;
use rapidhash::RapidHashMap;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::FsError;


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LangTemplateMap {
	/// Map of all templates where the key is the hash of the template.
	pub templates: RapidHashMap<LangContentHash, LangTemplate>,
}

impl std::ops::Deref for LangTemplateMap {
	type Target = RapidHashMap<LangContentHash, LangTemplate>;
	fn deref(&self) -> &Self::Target { &self.templates }
}
impl std::ops::DerefMut for LangTemplateMap {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.templates }
}


impl LangTemplateMap {
	pub fn new(templates: RapidHashMap<LangContentHash, LangTemplate>) -> Self {
		Self { templates }
	}
	/// Load the template map created by the beet cli.
	#[cfg(all(feature = "serde", not(target_arch = "wasm32")))]
	pub fn load(src: impl AsRef<std::path::Path>) -> Result<Self> {
		let tokens = sweet::prelude::ReadFile::to_string(src)?;
		let this = ron::de::from_str(&tokens.to_string())?;
		Result::Ok(this)
	}
}


impl Pipeline<WebNode, Result<WebNode>> for &LangTemplateMap {
	fn apply(self, mut node: WebNode) -> Result<WebNode> {
		let templates = self.extract_lang_template_directives(&mut node)?;
		self.insert_lang_templates(templates, &mut node);
		Ok(node)
	}
}

impl LangTemplateMap {
	/// Walk the tree and extract all the [`TemplateDirective::LangTemplate`] directives,
	/// removing them from the tree.
	/// In the case of style templates, style ids will be assingned.
	fn extract_lang_template_directives(
		&self,
		node: &mut WebNode,
	) -> Result<RapidHashMap<LangContentHash, &LangTemplate>> {
		// all templates used for this route
		let mut route_templates = RapidHashMap::default();

		let mut result = Ok(());
		// println!(
		// 	"visiting: {}",
		// 	node.clone().xpipe(RsxToHtmlString::default()).unwrap()
		// );
		VisitWebNodeMut::walk(node, |node| {
			let Some(content_hash) = node.lang_template() else {
				return;
			};
			if let Some(template) = self.templates.get(&content_hash) {
				// insert if doesnt exist
				route_templates.entry(content_hash).or_insert(template);
				// assign StyleId if it is a local style template
				if template.tag == "style"
					&& let StyleScope::Local =
						node.style_scope().unwrap_or_default()
				{
					node.push_directive(WebDirective::StyleId {
						id: template.id,
					});
				} else {
					// otherwise remove the node
					*node = Default::default();
				}
			} else {
				result = Err(LangTemplateMapError::Missing {
					span: node.meta().span().clone(),
					expected: content_hash,
				});
			}
		});
		result?;
		Ok(route_templates)
	}

	/// Insert each provided template into the node, appending the [`TemplateDirective::Head`] directive
	/// to each.
	fn insert_lang_templates(
		&self,
		templates: RapidHashMap<LangContentHash, &LangTemplate>,
		node: &mut WebNode,
	) {
		for template in templates.values() {
			let mut meta = template.create_meta();
			meta.push_directive(TemplateDirectiveEnum::Head);
			node.push(
				RsxElement {
					tag: template.tag.clone(),
					attributes: Vec::new(),
					children: Box::new(
						RsxText {
							meta: NodeMeta::new(
								meta.span().clone(),
								Vec::new(),
							),
							value: template.content.clone(),
						}
						.into(),
					),
					self_closing: false,
					meta,
				}
				.into(),
			)
		}
	}
}

type Result<T> = std::result::Result<T, LangTemplateMapError>;

#[derive(Debug, thiserror::Error)]
pub enum LangTemplateMapError {
	#[error("Missing LangTemplate\nSpan: {span}\nExpected: {expected}")]
	Missing {
		expected: LangContentHash,
		span: FileSpan,
	},
	#[error("Failed to deserialize\n{0}")]
	Deserialize(#[from] ron::de::SpannedError),
	#[error("\n{0}")]
	Fs(#[from] FsError),
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_common::prelude::*;
	use sweet::prelude::*;

	fn create_node(content_hash: u64) -> WebNode {
		RsxElement {
			tag: "style".to_string(),
			attributes: vec![],
			children: Default::default(),
			self_closing: false,
			meta: NodeMeta::default().with_template_directives(vec![
				TemplateDirectiveEnum::LangTemplate {
					content_hash: LangContentHash::new(content_hash),
				},
			]),
		}
		.into()
	}
	fn create_map() -> LangTemplateMap {
		LangTemplateMap::new(
			vec![(
				LangContentHash::new(0),
				LangTemplate::new(
					"div".to_string(),
					0,
					vec![],
					"foobar".to_string(),
					LangContentHash::new(0),
					vec![],
				),
			)]
			.into_iter()
			.collect(),
		)
	}

	#[test]
	fn missing_template() {
		create_node(38).xpipe(&create_map()).xpect().to_be_err();
	}
}
