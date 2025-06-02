use crate::prelude::*;
use beet_common::prelude::*;
use beet_parse::prelude::*;
use bevy::prelude::*;
use sweet::prelude::ReadFile;
use sweet::prelude::WorkspacePathBuf;
use syn::visit::Visit;


/// For a given rust file, extract tokens from `rsx!` macros and insert them
/// as [`RstmlTokens`].
pub fn templates_to_nodes_rs(
	_: TempNonSendMarker,
	mut commands: Commands,
	query: Populated<(Entity, &TemplateFile), Changed<TemplateFile>>,
) -> Result {
	let mac_ident = syn::parse_quote!(rsx);
	for (entity, path) in query.iter() {
		if let Some(ex) = path.extension()
			&& ex == "rs"
		{
			let file = ReadFile::to_string(path.into_abs_unchecked())?;
			let file = syn::parse_file(&file)?;
			RsxSynVisitor {
				parent: entity,
				commands: &mut commands,
				file: &*path,
				mac: &mac_ident,
				index: 0,
			}
			.visit_file(&file);
		}
	}
	Ok(())
}


/// Visit a file, extracting an [`FileSpan`] and [`WebNodeTemplate`] for each
/// `rsx!` macro in the file.
struct RsxSynVisitor<'a, 'w, 's> {
	parent: Entity,
	commands: &'a mut Commands<'w, 's>,
	/// Used for creating [`FileSpan`] in several places.
	/// We must use workspace relative paths because locations are created
	/// via the `file!()` macro.
	file: &'a WorkspacePathBuf,
	mac: &'a syn::Ident,
	/// the index used for building the [`TemplateKey`].
	index: usize,
}

impl<'a, 'w, 's> Visit<'a> for RsxSynVisitor<'a, 'w, 's> {
	fn visit_macro(&mut self, mac: &syn::Macro) {
		if mac
			.path
			.segments
			.last()
			.map_or(false, |seg| &seg.ident == self.mac)
		{
			let index = self.index;
			self.index += 1;


			// mac.tokens is the inner tokens of the macro, ie the foo in rsx!{foo}
			// important for tracking exact span of the macro
			let tokens = mac.tokens.clone();
			self.commands.spawn((
				TemplateFileSource(self.parent),
				SourceFile::new(self.file.clone()),
				RstmlTokens::new(tokens),
				TemplateKey::new(self.file.clone(), index),
			));
		}
	}
}
