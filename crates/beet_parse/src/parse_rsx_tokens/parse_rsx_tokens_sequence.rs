use super::*;
#[allow(unused)]
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;

/// A sequence for parsing raw rstml token streams and combinator strings into
/// rsx trees, then extracting directives.
pub struct ParseRsxTokensSequence;

impl ParseRsxTokensSequence {
	/// Spawn the bundle, run the function with it, then return the result.
	pub fn parse_and_run<O>(
		bundle: impl Bundle,
		func: impl FnOnce(&World, Entity) -> O,
	) -> Result<O> {
		let mut world = World::new();
		let entity = world.spawn(bundle).id();
		world.run_sequence_once(ParseRsxTokensSequence)?;
		let out = func(&mut world, entity);
		// world.despawn(entity);
		Ok(out)
	}
}


impl WorldSequence for ParseRsxTokensSequence {
	fn run_sequence<R: WorldSequenceRunner>(
		self,
		runner: &mut R,
	) -> Result<()> {
		let world = runner.world_mut();
		world.init_resource::<HtmlConstants>();
		let constants = world.resource::<HtmlConstants>();
		let rstml_parser = create_rstml_parser(constants);
		world.insert_non_send_resource(rstml_parser);

		(
			// parsing raw tokens
			#[cfg(feature = "rsx")]
			parse_combinator_tokens,
			#[cfg(feature = "rsx")]
			parse_rstml_tokens,
			// extractors
			// lang nodes must run first, hashes raw attributes not extracted directives
			extract_lang_nodes,
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
			#[cfg(feature = "css")]
			parse_lightning,
		)
			.run_sequence(runner)
	}
}
