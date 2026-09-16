use crate::style::*;
use beet_core::prelude::*;

/// The form a diagram (a ```` ```mermaid ```` fence, a `<Mermaid>`) takes: the
/// `diagram-render` cascade property. Inherited like a text prop, so it is set
/// per app (a rule), per page (`bx:style` on the page root) or per diagram (the
/// fence info word, ```` ```mermaid text ````).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DiagramRender {
	/// Resolved by sink: svg on the web; on a terminal, text for a flowchart
	/// (it reflows to the column budget) and a raster for every other type
	/// where the session's terminal has kitty graphics, else text.
	#[default]
	Auto,
	/// The rendered svg. A build without the svg renderer (wasm, `mermaid`
	/// without `mermaid_svg`) falls back to text with a warning, never an error:
	/// a lean binary keeps the diagram, only its form degrades.
	Svg,
	/// Unicode box-drawing text on every sink.
	Text,
}

impl DiagramRender {
	/// Parse a fence info word (`text`, `svg`, `auto`, any case).
	pub fn parse_word(word: &str) -> Option<Self> {
		match word.to_ascii_lowercase().as_str() {
			"auto" => Some(Self::Auto),
			"svg" => Some(Self::Svg),
			"text" => Some(Self::Text),
			_ => None,
		}
	}
}

impl AsCssValue for DiagramRender {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Auto => "auto",
			Self::Svg => "svg",
			Self::Text => "text",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

/// A paint role of a diagram: one of the `--diagram-*` custom properties the
/// cascade declares for a figure (`common_props`, the material defaults in
/// `parse/mermaid`). The svg theme writes each role as its `var()`, so the
/// picture follows the colour scheme live; a terminal resolves the same
/// properties to hex. Custom properties rather than the token variables because
/// the stylesheet builder renames those (`FormatVariables`) and an inline svg
/// attribute is outside its reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagramPaint {
	/// The picture's background and the mask behind edge labels,
	/// `--diagram-surface`.
	Surface,
	/// A node's fill, `--diagram-node-fill`.
	NodeFill,
	/// A node's label, `--diagram-node-text`.
	NodeText,
	/// Node, actor and note borders, `--diagram-outline`.
	Outline,
	/// Edges and arrowheads, `--diagram-line`.
	Line,
	/// Free text: titles, edge labels, legends, `--diagram-text`.
	Text,
	/// Secondary fills: activations, `--diagram-secondary-fill`.
	SecondaryFill,
	/// Tertiary fills: notes, tags, `--diagram-tertiary-fill`.
	TertiaryFill,
	/// Text on a tertiary fill, `--diagram-tertiary-text`.
	TertiaryText,
	/// Subgraph and commit label fills, `--diagram-cluster-fill`.
	ClusterFill,
	/// Subgraph borders, lifelines and dividers, `--diagram-cluster-outline`.
	ClusterOutline,
	/// The typeface, `--diagram-font`.
	Font,
	/// Step `n` (zero-based) of the ramp behind pie slices and git branches,
	/// `--diagram-ramp-{n+1}-fill`.
	RampFill(usize),
	/// Text on ramp step `n`, `--diagram-ramp-{n+1}-text`.
	RampText(usize),
}

impl DiagramPaint {
	/// The custom property this role is declared as.
	pub fn css_name(&self) -> SmolStr {
		match self {
			Self::Surface => "--diagram-surface".into(),
			Self::NodeFill => "--diagram-node-fill".into(),
			Self::NodeText => "--diagram-node-text".into(),
			Self::Outline => "--diagram-outline".into(),
			Self::Line => "--diagram-line".into(),
			Self::Text => "--diagram-text".into(),
			Self::SecondaryFill => "--diagram-secondary-fill".into(),
			Self::TertiaryFill => "--diagram-tertiary-fill".into(),
			Self::TertiaryText => "--diagram-tertiary-text".into(),
			Self::ClusterFill => "--diagram-cluster-fill".into(),
			Self::ClusterOutline => "--diagram-cluster-outline".into(),
			Self::Font => "--diagram-font".into(),
			Self::RampFill(step) => {
				format!("--diagram-ramp-{}-fill", step + 1).into()
			}
			Self::RampText(step) => {
				format!("--diagram-ramp-{}-text", step + 1).into()
			}
		}
	}

	/// The web's paint for this role, `var(--diagram-..)`.
	pub fn css_value(&self) -> String { format!("var({})", self.css_name()) }
}

/// The tonal ramp behind pie slices and git branches: a `(background,
/// foreground)` token pair per step, serialized as [`DiagramRamp::STEPS`]
/// `--diagram-ramp-n-fill`/`-text` pairs, cycling a shorter ramp.
#[derive(Debug, Clone, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DiagramRamp(pub Vec<ColorRole>);

impl DiagramRamp {
	/// The steps the stylesheet declares: the twelve pie slots.
	pub const STEPS: usize = 12;

	/// The `(background, foreground)` pair of `step`, cycling.
	pub fn step(&self, step: usize) -> Option<&ColorRole> {
		self.0.get(step % self.0.len().max(1))
	}
}

impl AsCssValues for DiagramRamp {
	fn suffixes() -> Vec<CssKey> {
		(0..Self::STEPS)
			.flat_map(|step| {
				[
					CssKey::Property(DiagramPaint::RampFill(step).css_name()),
					CssKey::Property(DiagramPaint::RampText(step).css_name()),
				]
			})
			.collect()
	}

	fn as_css_values(&self) -> Result<Vec<CssValue>> {
		if self.0.is_empty() {
			bevybail!("a diagram ramp needs at least one step");
		}
		(0..Self::STEPS)
			.flat_map(|step| {
				let role = self.step(step).unwrap();
				[
					CssVariable::from_token_key(role.background.key()).xinto(),
					CssVariable::from_token_key(role.foreground.key()).xinto(),
				]
			})
			.collect::<Vec<CssValue>>()
			.xok()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;
	use crate::style::common_props::DiagramRampProp;
	use crate::style::common_props::DiagramRenderProp;
	use crate::style::material::MaterialStylePlugin;
	use crate::style::material::colors;

	/// The ramp declares every step as a fill/text pair, cycling a short ramp,
	/// and each paint role's name is the property the stylesheet writes.
	#[beet_core::test]
	fn ramp_serializes_as_custom_properties() {
		let ramp = DiagramRamp(vec![
			ColorRole {
				background: colors::PrimaryContainer.into(),
				foreground: colors::OnPrimaryContainer.into(),
			},
			ColorRole {
				background: colors::SecondaryContainer.into(),
				foreground: colors::OnSecondaryContainer.into(),
			},
		]);
		let declarations = CssRule::from_rule(
			&common_props::token_map(),
			&Rule::new().with_value(DiagramRampProp, ramp),
		)
		.unwrap()
		.into_declarations();
		declarations.len().xpect_eq(DiagramRamp::STEPS * 2);
		declarations
			.get(&CssKey::Property(DiagramPaint::RampFill(2).css_name()))
			.unwrap()
			.to_string()
			.xpect_contains("primary-container");
		declarations
			.get(&CssKey::Property(DiagramPaint::RampText(11).css_name()))
			.unwrap()
			.to_string()
			.xpect_contains("on-secondary-container");
	}

	#[beet_core::test]
	fn serializes_as_a_custom_property() {
		CssRule::from_rule(
			&common_props::token_map(),
			&Rule::new().with_canonical(DiagramRender::Text),
		)
		.unwrap()
		.into_declarations()
		.into_iter()
		.map(|(key, value)| format!("{key}: {value}"))
		.collect::<Vec<_>>()
		.xpect_eq(vec!["--diagram-render: text".to_string()]);
	}

	/// A page root's `bx:style` reaches every figure below it (the prop is
	/// inherited), and a nested declaration overrides it.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn inherits_from_the_page_root() {
		let mut world = MaterialStylePlugin::world();
		let container = world
			.spawn_template(BsxTemplate::container(
				BsxNode::parse_document(
					r#"<div bx:style="diagram-render=Text"><figure/><div bx:style="diagram-render=Svg"><figure/></div></div>"#,
					&BsxParseConfig::bsx(),
				)
				.unwrap(),
				BsxTemplateRegistry::default(),
			))
			.unwrap()
			.id();
		world.flush();
		let page = world.entity(container).get::<Children>().unwrap()[0];
		let page_children =
			world.entity(page).get::<Children>().unwrap().to_vec();
		let nested_figure =
			world.entity(page_children[1]).get::<Children>().unwrap()[0];
		let resolve = |world: &mut World, entity: Entity| {
			world.with_state::<RuleSetQuery, _>(|rules| {
				rules
					.resolve(
						entity,
						DiagramRenderProp,
						&mut CascadeMemo::default(),
					)
					.unwrap()
			})
		};
		resolve(&mut world, page_children[0]).xpect_eq(DiagramRender::Text);
		resolve(&mut world, nested_figure).xpect_eq(DiagramRender::Svg);
	}
}
