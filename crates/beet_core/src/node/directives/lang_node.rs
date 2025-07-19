use crate::as_beet::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use std::hash::Hash;
use std::hash::Hasher;


/// The fs loaded and deduplicated [`InnerText`], existing seperately from the
/// originating tree(s).
/// Created alongside a [`NodeTag`], [`LangSnippetPath`] and optionally a [`StyleId`]
#[derive(Debug, Clone, PartialEq, Hash, Deref, Component, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component)]
// #[component(immutable)]
pub struct LangSnippet(pub String);

impl LangSnippet {
	/// Create a new [`LangSnippet`] from a `String`.
	pub fn new(content: impl Into<String>) -> Self { Self(content.into()) }
}

/// The replacement for [`InnerText`] after the lang snippet has been
/// extracted, referencing the path to the snippet scene file.
#[derive(Debug, Clone, PartialEq, Hash, Deref, Component, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component)]
// #[component(immutable)]
pub struct LangSnippetPath(pub WsPathBuf);


#[derive(Debug, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[require(InnerText)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct ScriptElement {
	/// The 'type' attribute of the `<script>` element, e.g. `type="module"`,
	/// defaults to "text/javascript"
	pub script_type: String,
}

#[derive(Debug, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[require(InnerText)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct StyleElement;

#[derive(Debug, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[require(InnerText)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct CodeElement {
	/// the 'lang' attribute of the `<code>` element, e.g. `lang="rust"`,
	/// defaults to "plaintext"
pub	lang: String,
}


/// Elements like `script`,`style` or `code` may contain either a single child
/// text node or a src attribute pointing to a file.
/// This directive contains the content of that element and is added *alongside*
/// the element.
#[derive(Debug, Default, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct InnerText(pub String);

/// An intermediate representation of an [`InnerText`] defined by a `src` attribute,
/// ie `<style src="style.css">`.
/// Upon tokenization this is replaced with an include_str,
/// ie [`InnerText(include_str!("style.css"))`],
/// feature gated behind a  [`not(feature="client")`] to avoid excessivly large
/// client bundles, otherwise inserting a placeholder comment,
/// ie `<!-- FileInnerText(style.css) -->`.
#[derive(Debug, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FileInnerText(
	/// path to the file relative to the source file,
	pub String,
);

#[cfg(feature = "tokens")]
impl TokenizeSelf for FileInnerText {
	fn self_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let path = &self.0;
		tokens.extend(quote::quote! {
			#[cfg(not(feature = "client"))]
			{
				InnerText::new(include_str!(#path))
			}
			#[cfg(feature = "client")]
			{
				InnerText::new(format!("<!-- FileInnerText({}) -->", #path))
			}
		});
	}
}

impl InnerText {
	/// Create a new [`InnerText`] from a `String`.
	pub fn new(text: impl Into<String>) -> Self { Self(text.into()) }

	/// create a hash ignoring whitespace in the case of [`Self::Inline`]
	pub fn hash_no_whitespace(&self, hasher: &mut impl Hasher) {
		self.0.replace(char::is_whitespace, "").hash(hasher);
	}
}

/// For script and style tags, replace the [`ElementNode`] with a [`InnerText`]
pub(crate) fn extract_lang_nodes(
	mut commands: Commands,
	text_nodes: Query<&TextNode>,
	attr_lits: Query<(Entity, &AttributeKey, Option<&AttributeLit>)>,
	query: Populated<
		(Entity, &NodeTag, Option<&Attributes>, Option<&Children>),
		Added<NodeTag>,
	>,
) {
	let find_attr = |attrs: &Option<&Attributes>,
	                 key: &str|
	 -> Option<(Entity, Option<&AttributeLit>)> {
		attrs.as_ref()?.iter().find_map(|entity| {
			let (attr_entity, inner_key, value) = attr_lits.get(entity).ok()?;
			if inner_key.as_str() == key {
				Some((attr_entity, value))
			} else {
				None
			}
		})
	};

	'iter_elements: for (entity, tag, attributes, children) in query.iter() {
		// entirely skip is:inline
		if find_attr(&attributes, "is:inline").is_some() {
			continue 'iter_elements;
		}

		// 1. Convert from 'ElementNode' to 'LangNode'
		match tag.as_str() {
			"script" => {
				let script_type = find_attr(&attributes, "type")
					.and_then(|(_, value)| value)
					.map_or_else(
						|| "text/javascript".to_string(),
						|lit| lit.to_string(),
					);

				commands
					.entity(entity)
					.insert(ScriptElement { script_type });
			}
			"style" => {
				commands.entity(entity).insert(StyleElement);
			}
			"code" => {
				let lang = find_attr(&attributes, "lang")
					.and_then(|(_, value)| value)
					.map_or_else(
						|| "plaintext".to_string(),
						|lit| lit.to_string(),
					);
				commands.entity(entity).insert(CodeElement { lang });
			}
			_ => {
				// skip non-lang nodes
				continue 'iter_elements;
			}
		}
		commands
			.entity(entity)
			.remove::<ElementNode>()
			.remove::<NodeTag>()
			.despawn_related::<Children>();

		// 1. Collect InnerText
		for child in children.iter().flat_map(|c| c.iter()) {
			if let Ok(text_node) = text_nodes.get(child) {
				commands
					.entity(entity)
					.insert(InnerText(text_node.text().to_string()));
				commands.entity(child).despawn();
				continue 'iter_elements;
			}
		}
		// 2. Collect FileInnerText
		if let Some((attr_entity, value)) = find_attr(&attributes, "src")
			&& let Some(AttributeLit::String(value)) = value
		{
			commands.entity(entity).insert(FileInnerText(value.clone()));
			commands.entity(attr_entity).despawn();
			continue 'iter_elements;
		}
		// 3. If no text or src, insert an empty InnerText
		commands.entity(entity).insert(InnerText::default());
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;


	#[test]
	fn extracts_inline() {
		let mut world = World::new();
		let entity = world
			.spawn((
				NodeTag::new("style"),
				FileSpanOf::<ElementNode>::new(FileSpan::default()),
				children![TextNode::new("div { color: red; }")],
			))
			.id();
		world.run_system_once(super::extract_lang_nodes).unwrap();
		let entity = world.entity(entity);
		entity
			.get::<InnerText>()
			.unwrap()
			.xpect()
			.to_be(&InnerText::new("div { color: red; }"));
		entity.contains::<Children>().xpect().to_be(false);
	}
	#[test]
	fn extracts_src() {
		let mut world = World::new();
		let entity = world
			.spawn((
				NodeTag::new("style"),
				FileSpanOf::<ElementNode>::new(FileSpan::new(
					file!(),
					default(),
					default(),
				)),
				related!(
					Attributes[(
						AttributeKey::new("src"),
						AttributeLit::String("./style.css".to_string())
					)]
				),
			))
			.id();
		world.run_system_once(super::extract_lang_nodes).unwrap();
		let entity = world.entity(entity);
		entity
			.get::<FileInnerText>()
			.unwrap()
			.xpect()
			.to_be(&FileInnerText("./style.css".to_string()));
		entity.contains::<Attributes>().xpect().to_be(false);
	}
}
