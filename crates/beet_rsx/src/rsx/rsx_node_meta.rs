use crate::prelude::*;



#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RsxNodeMeta {
	pub template_directives: Vec<TemplateDirective>,
	pub location: Option<RsxMacroLocation>,
}

impl NodeMeta for RsxNodeMeta {
	fn meta(&self) -> &RsxNodeMeta { self }
	fn meta_mut(&mut self) -> &mut RsxNodeMeta { self }
}

pub trait NodeMeta {
	fn meta(&self) -> &RsxNodeMeta;
	fn meta_mut(&mut self) -> &mut RsxNodeMeta;
	fn location(&self) -> Option<&RsxMacroLocation> {
		self.meta().location.as_ref()
	}
	fn with_location(mut self, location: RsxMacroLocation) -> Self
	where
		Self: Sized,
	{
		self.set_location(location);
		self
	}

	fn remove_location(&mut self) { self.meta_mut().location = None; }


	fn set_location(&mut self, location: RsxMacroLocation) {
		self.meta_mut().location = Some(location);
	}
	fn location_str(&self) -> String {
		match self.location() {
			Some(loc) => loc.to_string(),
			None => "<unknown>".to_string(),
		}
	}


	fn template_directives(&self) -> &Vec<TemplateDirective> {
		self.meta().template_directives.as_ref()
	}
	fn set_template_directives(&mut self, directives: Vec<TemplateDirective>) {
		self.meta_mut().template_directives = directives;
	}
	fn with_template_directives(
		mut self,
		directives: Vec<TemplateDirective>,
	) -> Self
	where
		Self: Sized,
	{
		self.set_template_directives(directives);
		self
	}
}


impl<T: NodeMeta> TemplateDirectiveExt for T {
	fn is_client_reactive(&self) -> bool {
		self.template_directives()
			.iter()
			.any(|d| d.is_client_reactive())
	}

	fn is_local_scope(&self) -> bool {
		self.template_directives()
			.iter()
			.any(|d| d.is_local_scope())
	}

	fn is_global_scope(&self) -> bool {
		self.template_directives()
			.iter()
			.any(|d| d.is_global_scope())
	}
}
