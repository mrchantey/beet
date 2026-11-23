use crate::prelude::*;

/// Registers the BSX resolver seams, the [`BsxConditions`] namespaces
/// `bx:cfg` evaluates against, and the core types the standard directives and
/// HTML's own elements build into, so markup reads them with nothing else
/// installed: a script, the event it answers, and a `<template>`'s
/// [`Prefab`] with its [`Held`].
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
			.init_resource::<TemplateFormats>()
			.init_resource::<BsxTemplateRegistry>()
			.register_type::<RunScript>()
			.register_type::<OnEvent>()
			.register_type::<Prefab>()
			.install_type::<Held>()
			// the marker a registry template leaves on the entity it built into.
			.register_type::<TemplateInstance>();
	}
}
