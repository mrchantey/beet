use crate::prelude::*;
use beet_bevy::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// The default codegen builder for a beet site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodegenWasmConfig {
	/// These imports will be added to the head of the wasm imports file.
	/// This will be required for any components with a client island directive.
	/// By default this will include `use beet::prelude::*;`
	#[serde(default = "default_wasm_imports", with = "syn_item_vec_serde")]
	pub wasm_imports: Vec<syn::Item>,
}
fn default_wasm_imports() -> Vec<syn::Item> {
	vec![syn::parse_quote!(
		use beet::prelude::*;
	)]
}

impl Default for CodegenWasmConfig {
	fn default() -> Self {
		Self {
			wasm_imports: default_wasm_imports(),
		}
	}
}

impl NonSendPlugin for CodegenWasmConfig {
	fn build(self, _app: &mut App) { todo!() }
}
