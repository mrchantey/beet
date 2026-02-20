use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::parse_macro_input;


pub fn impl_as_any(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let item = parse_macro_input!(item as DeriveInput);
	let result = parse_as_any(item);
	result.unwrap_or_else(|err| err.into_compile_error()).into()
}

fn parse_as_any(input: DeriveInput) -> Result<TokenStream, syn::Error> {
	let name = &input.ident;
	let (impl_generics, ty_generics, where_clause) =
		input.generics.split_for_impl();

	let expanded = quote! {
	 impl #impl_generics AsAny for #name #ty_generics #where_clause {
	  fn as_any(&self) -> &dyn std::any::Any {
	   self
	  }
	  fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
	   self
	  }
	  fn as_any_boxed(self: Box<Self>) -> Box<dyn std::any::Any> {
	   self
	  }
	 }
	};

	Ok(expanded)
}
