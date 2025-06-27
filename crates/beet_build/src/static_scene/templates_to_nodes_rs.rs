use crate::prelude::*;
use beet_common::prelude::*;
use beet_parse::prelude::*;
use beet_template::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use syn::visit::Visit;


/// For a given rust file, extract tokens from `rsx!` macros and insert them
/// as [`RstmlTokens`].
pub fn templates_to_nodes_rs(
	_: TempNonSendMarker,
	macros: Res<TemplateMacros>,
	mut commands: Commands,
	query: Populated<(Entity, &TemplateFile), Changed<TemplateFile>>,
) -> Result {
	for (entity, path) in query.iter() {
		if let Some(ex) = path.extension()
			&& ex == "rs"
		{
			let file = ReadFile::to_string(path.into_abs())?;
			let file = syn::parse_file(&file)?;
			RsxSynVisitor {
				parent: entity,
				commands: &mut commands,
				file: &*path,
				macros: &*macros,
			}
			.visit_file(&file);
		}
	}
	Ok(())
}

/// Spawn an [`RstmlTokens`] for each `rsx!` macro in the file.
struct RsxSynVisitor<'a, 'w, 's> {
	parent: Entity,
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
				ChildOf(self.parent),
				SourceFile::new(self.file.clone()),
				StaticNodeRoot,
				RstmlTokens::new(tokens),
			));
		}
	}
}
