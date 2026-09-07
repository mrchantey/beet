use crate::prelude::*;

/// Registers the BSX resolver seams: the event/verb registries so
/// `bx:<event>=verb{ arg: value, .. }` resolves at build time, and the
/// [`BsxConditions`] namespaces `bx:cfg` evaluates against.
///
/// The event and verb registries are **empty by default**: core knows no
/// concrete event or verb. An app (or `beet_ui`'s default registration)
/// installs the concrete `click` event installer and the example verb set.
/// [`BsxConditions`] is the exception: it ships the two facts every build has
/// (`feature:` and `env:`), since a condition that cannot be answered is an
/// error rather than a graceful no-op.
pub struct BsxPlugin;

impl Plugin for BsxPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<BsxConditions>()
			.init_resource::<EventRegistry>()
			.init_resource::<VerbRegistry>()
			.init_resource::<BsxTagResolvers>()
			.init_resource::<StyleResolver>()
			.init_resource::<TemplateFormats>()
			.init_resource::<BsxTemplateRegistry>();
	}
}
