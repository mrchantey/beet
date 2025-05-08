use crate::prelude::*;



#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeMeta {
	pub template_directives: Vec<TemplateDirective>,
	pub location: Option<FileSpan>,
}

impl NodeMeta {
	pub fn new(location: Option<FileSpan>) -> Self {
		Self {
			template_directives: Vec::new(),
			location,
		}
	}
}

impl GetNodeMeta for NodeMeta {
	fn meta(&self) -> &NodeMeta { self }
	fn meta_mut(&mut self) -> &mut NodeMeta { self }
}

pub trait GetNodeMeta {
	fn meta(&self) -> &NodeMeta;
	fn meta_mut(&mut self) -> &mut NodeMeta;
	fn location(&self) -> Option<&FileSpan> { self.meta().location.as_ref() }
	fn with_location(mut self, location: FileSpan) -> Self
	where
		Self: Sized,
	{
		self.meta_mut().location = Some(location);
		self
	}

	fn remove_location(&mut self) { self.meta_mut().location = None; }

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


#[cfg(feature = "tokens")]
impl crate::prelude::RustTokens for NodeMeta {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		let location = match self.location() {
			Some(loc) => {
				let loc = loc.into_rust_tokens();
				quote::quote! {Some(#loc)}
			}
			None => quote::quote! {None},
		};

		let template_directives = self
			.template_directives()
			.iter()
			.map(|d| d.into_rust_tokens());
		quote::quote! {
			NodeMeta {
				template_directives: vec![#(#template_directives),*],
				location: #location
			}
		}
	}
}

#[cfg(feature = "tokens")]
impl crate::prelude::RonTokens for NodeMeta {
	fn into_ron_tokens(&self) -> proc_macro2::TokenStream {
		let location = match self.location() {
			Some(loc) => {
				let loc = loc.into_ron_tokens();
				quote::quote! {Some(#loc)}
			}
			None => quote::quote! {None},
		};
		let template_directives = self
			.template_directives()
			.iter()
			.map(|d| d.into_ron_tokens());
		quote::quote! {
			NodeMeta(
				template_directives: [#(#template_directives),*],
				location: #location,
			)
		}
	}
}
