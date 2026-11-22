use crate::prelude::*;

/// Registers the BSX resolver seams and the [`BsxConditions`] namespaces
/// `bx:cfg` evaluates against.
///
/// The seams are **empty by default**: core knows no concrete tag or style
/// grammar, which a downstream layer registers. [`BsxConditions`] is the
/// exception: it ships the two facts every build has (`feature:` and `env:`),
/// since a condition that cannot be answered is an error rather than a
/// graceful no-op.
pub struct BsxPlugin;

impl Plugin for BsxPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<BsxConditions>()
			.init_resource::<BsxTagResolvers>()
			.init_resource::<StyleResolver>()
			.init_resource::<TemplateFormats>()
			.init_resource::<BsxTemplateRegistry>()
			// the marker a registry template leaves on the entity it built into.
			.register_type::<TemplateInstance>();
	}
}
