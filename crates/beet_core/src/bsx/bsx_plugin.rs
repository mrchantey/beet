use crate::prelude::*;

/// Registers the BSX event/verb seam resources so
/// `bx:<event>=verb{ arg: value, .. }` resolves at build time.
///
/// Both registries are **empty by default**: core knows no concrete event or
/// verb. An app (or `beet_ui`'s default registration) installs the concrete
/// `click` event installer and the example verb set.
pub struct BsxPlugin;

impl Plugin for BsxPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<EventRegistry>()
			.init_resource::<VerbRegistry>()
			.init_resource::<BsxTagResolvers>()
			.init_resource::<StyleResolver>()
			.init_resource::<TemplateFormats>()
			.init_resource::<BsxTemplateRegistry>();
	}
}
