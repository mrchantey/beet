use crate::prelude::*;



#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeMeta {
	pub template_directives: Vec<TemplateDirective>,
	pub location: Option<RsxMacroLocation>,
}

impl GetNodeMeta for NodeMeta {
	fn meta(&self) -> &NodeMeta { self }
	fn meta_mut(&mut self) -> &mut NodeMeta { self }
}

pub trait GetNodeMeta {
	fn meta(&self) -> &NodeMeta;
	fn meta_mut(&mut self) -> &mut NodeMeta;
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


impl<T: GetNodeMeta> TemplateDirectiveExt for T {
	fn find_directive(
		&self,
		mut func: impl FnMut(&TemplateDirective) -> bool,
	) -> Option<&TemplateDirective> {
		self.template_directives().iter().find(|d| func(d))
	}
	fn find_map_directive<U>(
		&self,
		mut func: impl FnMut(&TemplateDirective) -> Option<&U>,
	) -> Option<&U> {
		self.template_directives().iter().find_map(|d| func(d))
	}
}
