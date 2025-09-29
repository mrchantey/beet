use super::*;
#[allow(unused)]
use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use proc_macro2::TokenStream;

/// A sequence for parsing raw rstml token streams and combinator strings into
/// rsx trees, then extracting directives.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct ParseRsxTokens;

impl ParseRsxTokens {
	/// Spawn the bundle, run the function with it, then return the result.
	pub fn parse_and_run(
		bundle: impl Bundle,
		func: impl FnOnce(&World, Entity) -> Result<TokenStream>,
	) -> Result<TokenStream> {
		// TODO cost 100us creating an app per macro, we should cache thread
		// local app, wait for BeetMain pattern
		let mut app = App::new();
		app.add_plugins(ParseRsxTokensPlugin);
		let world = app.world_mut();
		let entity = world.spawn(bundle).id();
		world.run_schedule(ParseRsxTokens);
		let tokens = func(world, entity)?;

		let imports = dom_imports();

		quote::quote! {{
			#imports
			#tokens
		}}
		.xok()
	}
}

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
			.insert_schedule_before(Update, ParseRsxTokens)
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
