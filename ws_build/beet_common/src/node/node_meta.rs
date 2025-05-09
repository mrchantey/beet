use crate::prelude::*;



#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeMeta {
	directives: Vec<TemplateDirective>,
	span: FileSpan,
}

impl NodeMeta {
	pub fn new(span: FileSpan, directives: Vec<TemplateDirective>) -> Self {
		Self { span, directives }
	}
}

impl GetNodeMeta for NodeMeta {
	fn meta(&self) -> &NodeMeta { self }
	fn meta_mut(&mut self) -> &mut NodeMeta { self }
}

impl<T: GetNodeMeta> GetSpan for T {
	fn span(&self) -> &FileSpan { &self.meta().span }
	fn span_mut(&mut self) -> &mut FileSpan { &mut self.meta_mut().span }
}

pub trait GetNodeMeta {
	fn meta(&self) -> &NodeMeta;
	fn meta_mut(&mut self) -> &mut NodeMeta;
	fn with_span(mut self, span: FileSpan) -> Self
	where
		Self: Sized,
	{
		self.meta_mut().span = span;
		self
	}
	fn directives(&self) -> &[TemplateDirective] {
		&self.meta().directives
	}
	fn push_directive(&mut self, directive: TemplateDirective) {
		self.meta_mut().directives.push(directive);
	}

	fn set_template_directives(&mut self, directives: Vec<TemplateDirective>) {
		self.meta_mut().directives = directives;
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

impl From<FileSpan> for NodeMeta {
	fn from(span: FileSpan) -> Self { Self::new(span, Vec::new()) }
}


impl<T: GetNodeMeta> TemplateDirectiveExt for T {
	fn find_directive(
		&self,
		mut func: impl FnMut(&TemplateDirective) -> bool,
	) -> Option<&TemplateDirective> {
		self.meta().directives.iter().find(|d| func(d))
	}
	fn find_map_directive<U>(
		&self,
		mut func: impl FnMut(&TemplateDirective) -> Option<&U>,
	) -> Option<&U> {
		self.meta().directives.iter().find_map(|d| func(d))
	}
}


#[cfg(feature = "tokens")]
impl crate::prelude::RustTokens for NodeMeta {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		let span = self.span.into_rust_tokens();
		let directives = self.directives.iter().map(|d| d.into_rust_tokens());
		quote::quote! {
			NodeMeta::new(#span,vec![#(#directives),*])
		}
	}
}
