use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::html::ClassStyle;
use syntect::html::ClassedHTMLGenerator;
use syntect::html::IncludeBackground;
use syntect::html::append_highlighted_html_for_styled_line;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Parse the output of pulldown-cmark into a `CodeNode`:
///
/// The following markdown:
/// ```markdown
/// 	```rust
/// 	let foo = bar;
/// 	```
/// ```
/// Will produce the following html:
/// ```html
/// <pre><code class="language-rust">let foo = bar;</code></pre>
pub fn collect_md_code_nodes(
	mut commands: Commands,
	query: Query<(Entity, &NodeTag, &Children), Added<ElementNode>>,
	mut inner_text: Query<&mut InnerText>,
	find_attr: FindAttribute,
) {
	for (entity, tag, children) in query.iter() {
		if **tag != "pre" {
			continue;
		}

		if children.len() != 1 {
			continue;
		}
		let Some(&child) = children.first() else {
			continue;
		};

		let lang = find_attr
			.classes(child)
			.into_iter()
			.find(|c| c.starts_with("language-"))
			.map(|c| c.trim_start_matches("language-").to_string());

		let theme = find_attr
			.find(entity, "theme")
			.and_then(|(_, value)| value.map(|v| v.as_str().to_string()));

		commands.entity(entity).insert(CodeNode { lang, theme });

		if let Ok(mut text) = inner_text.get_mut(child) {
			let text = std::mem::take(&mut text.0);
			// pulldown-cmark escapes code html, we undo this to avoid double escaping
			// by syntect
			let unescaped = EscapeHtml::unescape(&text);
			commands.entity(entity).insert(InnerText(unescaped));
			commands.entity(child).despawn();
		}
	}
}


#[derive(Debug, Resource)]
pub struct SyntectConfig {
	pub syntax_set: SyntaxSet,
	pub theme_set: ThemeSet,
	/// The default theme to use for highlighting.
	pub theme: String,
	/// The default language to use for highlighting.
	pub lang: String,
	/// The styling method to use for highlighting.
	pub styling: SyntectStyling,
}

#[derive(Debug, Default)]
pub enum SyntectStyling {
	/// Use inline styles for styling.
	#[default]
	Inline,
	/// Use classes for styling.
	Classes,
}

impl Default for SyntectConfig {
	fn default() -> Self {
		Self {
			syntax_set: SyntaxSet::load_defaults_newlines(),
			theme_set: ThemeSet::load_defaults(),
			styling: Default::default(),
			theme: "base16-ocean.dark".to_string(),
			lang: "plaintext".to_string(),
		}
	}
}


fn lang_aliases(lang: &str) -> &str {
	match lang {
		"txt" => "plain text",
		"text" => "plain text",
		"plaintext" => "plain text",
		// toml not yet supported
		"toml" => "yaml",
		"jsx" => "javascript",
		other => other,
	}
}

impl SyntectConfig {
	fn get_theme(
		&self,
		code_node: &CodeNode,
	) -> Result<&syntect::highlighting::Theme> {
		let theme_name = code_node.theme.as_deref().unwrap_or(&self.theme);
		self.theme_set.themes.get(theme_name).ok_or_else(|| {
			bevyhow!(
				"Failed to find theme: {}, available themes: {:?}",
				theme_name,
				self.theme_set.themes.keys()
			)
		})
	}
	fn get_lang(
		&self,
		code_node: &CodeNode,
	) -> Result<&syntect::parsing::SyntaxReference> {
		let lang = code_node.lang.as_deref().unwrap_or(&self.lang);
		let parsed_lang = lang_aliases(lang);
		self.syntax_set
			.find_syntax_by_token(parsed_lang)
			.ok_or_else(|| {
				let available_sets = self
					.syntax_set
					.syntaxes()
					.iter()
					.map(|s| &s.name)
					.collect::<Vec<_>>();
				bevyhow!(
					"Failed to find syntax for language: {}\nAvailable syntaxes: {:#?}",
					lang,
					available_sets
				)
			})
	}

	fn parse(&self, code_node: &CodeNode, text: &str) -> Result<String> {
		match self.styling {
			SyntectStyling::Classes => self.parse_classes(code_node, text),
			SyntectStyling::Inline => self.parse_inline(code_node, text),
		}
	}

	/// Parse the code node and return the highlighted HTML,
	/// this assumes the parent element has a background color matching the theme.
	fn parse_inline(&self, code_node: &CodeNode, text: &str) -> Result<String> {
		let theme = self.get_theme(code_node)?;
		let syntax = self.get_lang(code_node)?;
		let mut highlighter = HighlightLines::new(syntax, theme);
		let background = theme
			.settings
			.background
			.unwrap_or(syntect::highlighting::Color::WHITE);

		let mut output = String::new();
		for line in LinesWithEndings::from(text) {
			let regions = highlighter.highlight_line(line, &self.syntax_set)?;
			append_highlighted_html_for_styled_line(
				&regions[..],
				IncludeBackground::IfDifferent(background),
				&mut output,
			)?;
		}
		Ok(output)
	}

	/// Use classes to style the code node, the theme is not applied here.
	fn parse_classes(
		&self,
		code_node: &CodeNode,
		value: &str,
	) -> Result<String> {
		let syntax = self.get_lang(code_node)?;
		let mut generator = ClassedHTMLGenerator::new_with_class_style(
			syntax,
			&self.syntax_set,
			ClassStyle::Spaced,
		);
		for line in LinesWithEndings::from(value) {
			generator
				.parse_html_for_line_which_includes_newline(line)
				.unwrap();
		}
		let output = generator.finalize();
		Ok(output)
	}
}


pub fn parse_syntect(
	config: Res<SyntectConfig>,
	mut commands: Commands,
	mut query: Populated<(Entity, &CodeNode, &mut InnerText), Added<CodeNode>>,
) -> Result {
	for (entity, code_node, mut text) in query.iter_mut() {
		text.0 = config.parse(code_node, &text.0)?;

		commands.entity(entity).with_related::<AttributeOf>((
			AttributeKey::new("class"),
			NodeExpr::new(syn::parse_quote! {"syntect-code"}),
			TextNode::new("syntect-code"),
		));
	}
	Ok(())
}






#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_dom::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();
		world.init_resource::<SyntectConfig>();
		let entity = world
			.spawn((
				CodeNode::new("rust"),
				InnerText("fn foobar() -> String\n{}".to_string()),
			))
			.id();
		world.run_system_cached::<(), _, _>(parse_syntect).unwrap();

		world
			.entity(entity)
			.get::<InnerText>()
			.unwrap()
			.0
			.clone()
			.xpect_snapshot();
	}
}
