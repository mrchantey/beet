use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

#[derive(Default)]
pub struct StylePlugin;

impl Plugin for StylePlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<TokenPlugin>()
			.init_plugin::<ParsePlugin>()
			.init_resource::<CssTokenMap>()
			.register_type::<ColorScheme>();

		// mirror the typed scheme handle onto classes before the cascade runs,
		// so a runtime scheme switch re-themes on non-web targets.
		app.add_systems(
			PostParseTree,
			sync_color_scheme.before(ResolveStylesSet),
		);

		// terminal/char-cell defaults for prose elements (em → italic,
		// h1 → bold colour, …), expressed as ordinary tag rules.
		app.world_mut()
			.get_resource_or_init::<RuleSet>()
			.extend_rules(default_element_rules());

		#[cfg(all(feature = "syntax_highlighting", not(target_arch = "wasm32")))]
		{
			// highlight code blocks into styled spans, then resolve styles
			app.init_resource::<SyntaxHighlighting>().add_systems(
				PostParseTree,
				(
					apply_syntax_highlighting,
					resolve_styles.in_set(ResolveStylesSet),
				)
					.chain(),
			);
			// register the default theme so `.hl-<capture>` classes emitted by
			// `apply_syntax_highlighting` resolve to a foreground colour with no
			// further setup: each class rule redirects `color` to a syntax
			// token whose value lives in the root rule's declarations.
			let mut rules =
				app.world_mut().get_resource_or_init::<RuleSet>();
			rules.default_rule_mut().push_declarations(
				Rule::new().with_extend(syntax::default_scheme()),
			);
			rules.extend_rules(syntax::class_rules());
		}
		#[cfg(any(not(feature = "syntax_highlighting"), target_arch = "wasm32"))]
		app.add_systems(
			PostParseTree,
			resolve_styles.in_set(ResolveStylesSet),
		);
	}
}


/// The [`PostParseTree`] set that resolves [`VisualStyle`](crate::style::VisualStyle),
/// [`LayoutStyle`](crate::style::LayoutStyle), and [`BoxStyle`](crate::style::BoxStyle)
/// from the [`RuleSet`] cascade. Charcell decorations and the paint pipeline
/// run after it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ResolveStylesSet;
