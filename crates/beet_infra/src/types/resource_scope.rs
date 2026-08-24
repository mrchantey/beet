//! The one composition that turns a resource *label* into a resource *name*.

use crate::prelude::*;
use beet_core::prelude::*;

/// The app identity and stage a resource label is composed against, ie the
/// `beet-site` + `prod` that turn `analytics` into `beet-site--prod--analytics`.
///
/// A declaration carries only its label, and both meanings of that declaration
/// resolve the name here: the deploy (which creates the resource) and the
/// runtime (which reads or writes it). One composition, so the two cannot drift.
#[derive(Debug, Clone, PartialEq, Eq, Get)]
pub struct ResourceScope {
	app_name: SmolStr,
	stage: SmolStr,
}

impl ResourceScope {
	pub fn new(
		app_name: impl Into<SmolStr>,
		stage: impl Into<SmolStr>,
	) -> Self {
		Self {
			app_name: app_name.into(),
			stage: stage.into(),
		}
	}

	/// The process scope: the app identity from the world's [`PackageConfig`],
	/// the stage from this launch's [`BootstrapConfig`]. The fallback for a
	/// declaration made outside any deploy [`Stack`], ie one authored at router
	/// level purely for its runtime meaning.
	pub fn from_package(package: Option<&PackageConfig>) -> Result<Self> {
		let Some(app_name) = package.and_then(|package| package.app_name())
		else {
			bevybail!(
				"a resource declaration outside a `Stack` resolves its name from the app `<PackageConfig app_name=\"..\"/>`, which is not set"
			);
		};
		Self::new(app_name, BootstrapConfig::get().stage.clone()).xok()
	}

	/// The identifier a resource label composes to in this scope, the single
	/// definition of the `app--stage--label` convention.
	pub fn resource_ident(&self, label: impl Into<SmolStr>) -> terra::Ident {
		terra::Ident::new(self.app_name.clone(), self.stage.clone(), label)
	}

	/// The provider-facing resource name, ie `beet-site--prod--analytics`.
	pub fn resource_name(&self, label: impl Into<SmolStr>) -> String {
		self.resource_ident(label).primary_identifier().to_string()
	}
}

/// Resolves the [`ResourceScope`] for an entity: the nearest ancestor [`Stack`]
/// (so a stage override like the shared assets host wins), else the process
/// scope.
#[derive(SystemParam)]
pub struct ResourceScopeQuery<'w, 's> {
	stacks: AncestorQuery<'w, 's, &'static Stack>,
	package: Option<Res<'w, PackageConfig>>,
}

impl<'w, 's> ResourceScopeQuery<'w, 's> {
	pub fn get(&self, entity: Entity) -> Result<ResourceScope> {
		match self.stack(entity) {
			Some(stack) => stack.scope().xok(),
			None => ResourceScope::from_package(self.package.as_deref()),
		}
	}

	/// The deploy [`Stack`] this entity sits under, if any. A declaration made
	/// purely for its runtime meaning has none.
	pub fn stack(&self, entity: Entity) -> Option<&Stack> {
		self.stacks.get(entity).ok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The composition is the one the live stacks already resolve; renaming a
	/// deployed resource is a production incident, so these strings are pinned.
	#[beet_core::test]
	fn composes_the_live_names() {
		let prod = ResourceScope::new("beet-site", "prod");
		prod.resource_name("analytics")
			.xpect_eq("beet-site--prod--analytics");
		prod.resource_name("app").xpect_eq("beet-site--prod--app");
		ResourceScope::new("beet-site", "dev")
			.resource_name("artifacts")
			.xpect_eq("beet-site--dev--artifacts");
		ResourceScope::new("beet-site", "shared")
			.resource_name("assets")
			.xpect_eq("beet-site--shared--assets");
	}

	/// A [`Stack`] is a scope plus deploy machinery, so it composes identically.
	#[beet_core::test]
	fn stack_delegates_to_the_scope() {
		let stack = Stack::new("beet-site").with_stage("prod");
		stack
			.resource_ident("analytics")
			.primary_identifier()
			.as_str()
			.xpect_eq(
				ResourceScope::new("beet-site", "prod")
					.resource_name("analytics"),
			);
	}

	/// The process fallback needs an app identity: silently composing a name
	/// from a default is exactly the drift this type exists to prevent.
	#[beet_core::test]
	fn unnamed_package_is_loud() {
		ResourceScope::from_package(Some(&PackageConfig::default()))
			.unwrap_err()
			.to_string()
			.xpect_contains("app_name");
	}
}
