use super::*;
#[allow(unused)]
use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;


/// A system set for modifying the rsx tree after parsing tokens and extracting directives,
/// but before running additional parsers like `lightning`.
/// This gives downstream plugins a chance to modify the rsx tree
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ModifyRsxTree;

#[derive(Debug, Default, Clone)]
pub struct ParseRsxTokensPlugin;

impl Plugin for ParseRsxTokensPlugin {
	fn build(&self, app: &mut App) {
		let constants = app
			.init_resource::<HtmlConstants>()
			.world()
			.resource::<HtmlConstants>();
		let rstml_parser = create_rstml_parser(constants);

		#[cfg(feature = "syntect")]
		app.init_resource::<SyntectConfig>();

		app.insert_non_send_resource(rstml_parser)
			.configure_sets(ParseRsxTokens, ModifyRsxTree)
			.add_systems(
				ParseRsxTokens,
				(
					(
						// parsing raw tokens
						#[cfg(feature = "rsx")]
						parse_combinator_tokens,
						#[cfg(feature = "rsx")]
						parse_rstml_tokens,
						// extractors
						extract_inner_text_file,
						extract_inner_text_element,
						extract_inner_text_directive,
						extract_lang_nodes,
						collect_md_code_nodes,
						extract_code_nodes,
						extract_slot_targets,
						try_extract_directive::<SlotChild>,
						try_extract_directive::<ClientLoadDirective>,
						try_extract_directive::<ClientOnlyDirective>,
						try_extract_directive::<HtmlHoistDirective>,
						try_extract_directive::<StyleScope>,
						try_extract_directive::<StyleCascade>,
						// collect combinator exprs last
						#[cfg(feature = "rsx")]
						collapse_combinator_exprs,
					)
						.chain()
						.before(ModifyRsxTree),
					(
						#[cfg(feature = "syntect")]
						parse_syntect,
						#[cfg(feature = "css")]
						parse_lightning,
					)
						.chain()
						.after(ModifyRsxTree),
				),
			);
	}
}
