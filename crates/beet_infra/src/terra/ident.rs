use crate::terra::Resource;
use crate::terra::*;
use beet_core::prelude::*;
use heck::ToSnakeCase;

#[derive(Debug, Clone, Get)]
pub struct ResourceDef<T> {
	ident: Ident,
	resource: T,
}

impl<T> ResourceDef<T>
where
	T: Resource,
{
	/// Creates a new `ResourceDef` with the given identifier and resource.
	/// The resource's name is set to the primary identifier of the `Ident`.
	pub fn new_primary(ident: Ident, mut resource: T) -> Self
	where
		T: PrimaryResource,
	{
		resource.set_primary_identifier(ident.primary_identifier());
		Self { ident, resource }
	}

	pub fn new_secondary(ident: Ident, resource: T) -> Self {
		Self { ident, resource }
	}

	/// The terraform syntax for referencing a resource field,
	/// in the format `resorce_type.label_name.field_name`
	/// ie `aws_iam_role.lambda_role.name`
	pub fn field(&self, field_name: &str) -> String {
		let label = &self.ident.label;
		let resource_type = self.resource.resource_type();
		format!("{}.{}.{}", resource_type, label, field_name)
	}

	/// The terraform syntax for referencing a resource field in an interpolated string
	pub fn field_ref(&self, field_name: &str) -> String {
		format!("${{{}}}", self.field(field_name))
	}
}

/// All the components used to construct various resource identifiers.
#[derive(Debug, Clone, Get)]
pub struct Ident {
	app_name: SmolStr,
	stage: SmolStr,
	// label: SmolStr,
	/// kebab-case identifier for a resource,
	/// preferred by proivders
	/// ie:
	/// - bucket name
	/// - lambda function name
	/// - iam name
	primary_identifier: String,
	/// snake_case label for a resource, preferred for codegen and human readability
	/// ie:
	/// - terraform labels
	label: String,
}

impl Ident {
	pub fn new(
		app_name: impl Into<SmolStr>,
		stage: impl Into<SmolStr>,
		label_suffix: impl Into<SmolStr>,
	) -> Self {
		use heck::ToKebabCase;
		let app_name = app_name.into();
		let stage = stage.into();
		let label_suffix = label_suffix.into();

		let label = vec![&app_name, &stage, &label_suffix]
			.join("__")
			.to_snake_case();
		let primary_identifier = label.to_kebab_case();
		Self {
			app_name,
			stage,
			label,
			primary_identifier,
		}
	}
}
