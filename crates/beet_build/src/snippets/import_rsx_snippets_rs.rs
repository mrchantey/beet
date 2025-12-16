use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use beet_parse::prelude::*;
use syn::visit::Visit;


/// For a given rust file, extract tokens from `rsx!` macros and insert them
/// as [`RstmlTokens`].
pub fn import_rsx_snippets_rs(
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

/// Spawn an [`RstmlTokens`] for each `rsx!` macro in the file.
struct RsxSynVisitor<'a, 'w, 's> {
	source_file: Entity,
	commands: &'a mut Commands<'w, 's>,
	/// Used for creating [`FileSpan`] in several places.
	/// We must use workspace relative paths because locations are created
	/// via the `file!()` macro.
	file: &'a WsPathBuf,
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
	use sweet::prelude::*;

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
