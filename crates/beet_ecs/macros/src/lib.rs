mod inspector_options;
mod utils;

#[proc_macro_derive(InspectorOptions, attributes(inspector))]
pub fn inspectable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	inspector_options::inspectable(input)
}
