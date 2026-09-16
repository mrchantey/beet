use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

#[derive(Default)]
pub struct StylePlugin;

impl Plugin for StylePlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<TokenPlugin>()
			.init_plugin::<ParsePlugin>()
			.init_plugin::<CssPlugin>()
			.register_type::<ColorScheme>();

		// mirror the typed scheme handle onto classes before the cascade runs,
		// so a runtime scheme switch re-themes on non-web targets.
		app.add_systems(
			PostParseTree,
			sync_color_scheme.before(ResolveStylesSet),
		);

		// ease displayed styles toward the cascade's resolved targets. `Time`
		// is initialized here so a host without `TimePlugin` (eg a one-shot
		// render) still builds; its transitions simply never advance.
		app.init_resource::<Time>()
			.configure_sets(
				PostParseTree,
				AnimateStylesSet.after(ResolveStylesSet),
			)
			.add_systems(
				PostParseTree,
				animate_visual_transitions.in_set(AnimateStylesSet),
			);

		// terminal/char-cell defaults for prose elements (em → italic,
		// a → underline, …), expressed as ordinary tag rules.
		app.world_mut()
			.get_resource_or_init::<RuleSet>()
			.extend_rules(default_element_rules());

		// the cascade, with the fence passes ahead of it: a mermaid fence becomes
		// a figure before the highlighter could tokenize it (`DiagramSet`), and
		// the spans and figures both get styled by the one pass
		app.configure_sets(PostParseTree, DiagramSet.before(ResolveStylesSet))
			.add_systems(
				PostParseTree,
				resolve_styles.in_set(ResolveStylesSet),
			);

		#[cfg(feature = "mermaid")]
		{
			app.register_type::<MermaidDiagram>().add_systems(
				PostParseTree,
				(collect_mermaid_blocks, materialize_diagrams)
					.chain()
					.in_set(DiagramSet),
			);
			// the `:root` defaults, so the stylesheet states `--diagram-render:
			// auto` for a script to read, the cascade always resolves a mode, and
			// every paint role has its material colour for the svg's `var()`s
			let mut rules = app.world_mut().get_resource_or_init::<RuleSet>();
			rules.default_rule_mut().push_declarations(
				diagram_paint_defaults().with_canonical(DiagramRender::Auto),
			);
			rules.extend_rules(diagram_rules());
			// a page built again lays out only the diagrams that changed
			#[cfg(all(feature = "mermaid_svg", not(target_arch = "wasm32")))]
			app.init_resource::<DiagramSvgCache>();
		}

		#[cfg(all(
			feature = "syntax_highlighting",
			not(target_arch = "wasm32")
		))]
		{
			// highlight code blocks into styled spans, then resolve styles
			app.init_resource::<SyntaxHighlighting>().add_systems(
				PostParseTree,
				apply_syntax_highlighting
					.after(DiagramSet)
					.before(ResolveStylesSet),
			);
			// register the default theme so `.hl-<capture>` classes emitted by
			// `apply_syntax_highlighting` resolve to a foreground colour with no
			// further setup: each class rule redirects `color` to a syntax
			// token whose value lives in the root rule's declarations.
			let mut rules = app.world_mut().get_resource_or_init::<RuleSet>();
			rules.default_rule_mut().push_declarations(
				Rule::new().with_extend(syntax::default_scheme()),
			);
			rules.extend_rules(syntax::class_rules());
			// scheme-aware overrides: the `.light-scheme`/`.dark-scheme` body class
			// re-tints the highlight tokens (the root default is the dark palette,
			// unreadable on a light background).
			rules.extend_rules(vec![
				syntax::light_scheme(),
				syntax::dark_scheme(),
			]);
			// register the syntax tokens' CSS resolvers so the web `Stylesheet`
			// can serialize the `.hl-<capture>` colour variables (otherwise it
			// errors with "no CSS resolver registered" once a code block emits them).
			app.world_mut()
				.get_resource_or_init::<CssTokenMap>()
				.extend(syntax::token_map());
		}
	}
}

/// The [`PostParseTree`] set the diagram passes run in, ahead of the syntax
/// highlighter and [`ResolveStylesSet`]: a fence becomes a figure before the
/// highlighter could tokenize it. Empty without the `mermaid` feature.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct DiagramSet;

/// The [`PostParseTree`] set that resolves [`VisualStyle`](crate::style::VisualStyle),
/// [`LayoutStyle`](crate::style::LayoutStyle), and [`BoxStyle`](crate::style::BoxStyle)
/// from the [`RuleSet`] cascade. Charcell decorations and the paint pipeline
/// run after it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ResolveStylesSet;
