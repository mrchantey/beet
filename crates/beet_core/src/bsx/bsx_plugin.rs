use crate::prelude::*;

/// Registers the BSX resolver seams: the event registry so
/// `bx:<event>="script"` resolves at build time, and the [`BsxConditions`]
/// namespaces `bx:cfg` evaluates against.
///
/// The event registry is **empty by default**: core knows no concrete event,
/// and no way to run a script. An app (or `beet_ui`'s default registration)
/// installs the concrete `click` event installer.
/// [`BsxConditions`] is the exception: it ships the two facts every build has
/// (`feature:` and `env:`), since a condition that cannot be answered is an
/// error rather than a graceful no-op.
pub struct BsxPlugin;

impl Plugin for BsxPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<BsxConditions>()
			.init_resource::<EventRegistry>()
			.init_resource::<BsxTagResolvers>()
			.init_resource::<StyleResolver>()
			.init_resource::<TemplateFormats>()
			.init_resource::<BsxTemplateRegistry>();
	}
}
