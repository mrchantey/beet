//! RSX snippet extraction from Rust source files.
//!
//! This module handles parsing Rust files to extract `rsx!` macro invocations
//! and spawn them as [`RstmlTokens`] entities for further processing.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use beet_parse::prelude::*;
use syn::visit::Visit;


/// Extracts RSX snippets from Rust source files.
///
/// For each Rust file with the `.rs` extension, this system parses the file
/// and extracts tokens from `rsx!` macro invocations, spawning them as
/// child entities with [`RstmlTokens`] components.
pub(crate) fn import_rsx_snippets_rs(
	// even though our tokens are Unspan, they will be parsed by ParseRsxTokens
	// which also handles !Send tokens, so we must ensure main thread.
	_: TempNonSendMarker,
	macros: Res<TemplateMacros>,
	mut commands: Commands,
	query: Populated<(Entity, &SourceFile), Added<SourceFile>>,
) -> Result {
	for (entity, path) in query.iter() {
		if let Some(ex) = path.extension()
			&& ex == "rs"
		{
			trace!("rust source file changed: {}", path.display());

			let file = fs_ext::read_to_string(path)?;
			let file = syn::parse_file(&file)?;
			RsxSynVisitor {
				source_file: entity,
				commands: &mut commands,
				file: &path.into_ws_path()?,
				macros: &*macros,
			}
			.visit_file(&file);
		}
	}
	Ok(())
}

/// Visitor that spawns an [`RstmlTokens`] entity for each `rsx!` macro in a file.
struct RsxSynVisitor<'a, 'w, 's> {
	/// The parent source file entity.
	source_file: Entity,
	/// Commands for spawning child entities.
	commands: &'a mut Commands<'w, 's>,
	/// Workspace-relative path used for creating [`FileSpan`] in several places.
	/// We must use workspace relative paths because locations are created
	/// via the `file!()` macro.
	file: &'a WsPathBuf,
	/// Configuration for which macro names to recognize.
	macros: &'a TemplateMacros,
}

impl<'a, 'w, 's> Visit<'a> for RsxSynVisitor<'a, 'w, 's> {
	fn visit_macro(&mut self, mac: &syn::Macro) {
		if mac
			.path
			.segments
			.last()
			.map_or(false, |seg| *&seg.ident == *self.macros.rstml)
		{
			// mac.tokens is the inner tokens of the macro, ie the foo in rsx!{foo}
			// important for tracking exact span of the macro
			let tokens = mac.tokens.clone();
			self.commands.spawn((
				SnippetRoot::new_from_tokens(self.file.clone(), &tokens),
				StaticRoot,
				ChildOf(self.source_file),
				RstmlTokens::new(tokens),
			));
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_rsx::prelude::*;

	#[test]
	fn works() {
		let mut world = BuildPlugin::world();
		let test_site_index = WsPathBuf::new("tests/test_site/pages/index.rs");
		let entity = world
			.spawn(SourceFile::new(test_site_index.into_abs()))
			.id();

		world.run_schedule(ParseSourceFiles);
		let child = world.entity(entity).get::<Children>().unwrap()[0];
		world
			.run_system_cached_with(render_fragment, child)
			.unwrap()
			// only the output of the snippet, not the instance
			.xpect_str("party time!");
	}
}
