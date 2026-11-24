use crate::prelude::*;

/// Registers the [`BsxConditions`] namespaces `bx:cfg` evaluates against, and
/// the core types the standard directives and
/// HTML's own elements build into, so markup reads them with nothing else
/// installed: a script, the event it answers, and a `<template>`'s
/// [`Prefab`] with its [`Held`].
///
/// [`BsxConditions`] ships the facts every build has (`feature:`, `version:`
/// and `env:`), since a condition that cannot be answered is an error rather
/// than a graceful no-op.
#[derive(Default)]
pub struct BsxPlugin;

impl Plugin for BsxPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<AppTypeRegistry>()
			.init_resource::<BsxConditions>()
			.register_type::<Element>()
			.register_type::<Attribute>()
			.register_type::<AttributeOf>()
			.register_type::<Comment>()
			.register_type::<Doctype>()
			.register_type::<CData>()
			.register_type::<ProcessingInstruction>()
			.register_type::<Value>()
			.register_type::<SlotTarget>()
			.register_type::<SlotChild>()
			.register_type::<ChildOf>()
			.register_type::<FieldRef>()
			.register_type::<SourceFieldRef>()
			.register_type::<ResourceFieldRef>()
			.register_type::<ReflectFieldRef>()
			.register_type::<DocumentScope>()
			.register_type::<ReactiveChildren>()
			.register_type::<Cfg>()
			.register_type::<CfgExcluded>()
			.register_type::<Instance>()
			.register_type::<Prop>()
			.register_type::<RunScript>()
			.register_type::<OnEvent>()
			.register_type::<Prefab>()
			.install_type::<Held>();
	}
}
