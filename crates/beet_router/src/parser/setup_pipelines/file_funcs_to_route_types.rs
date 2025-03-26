// use crate::parser::FileGroup;
// use anyhow::Result;
// use beet_rsx::rsx::RsxPipeline;
// use beet_rsx::rsx::RsxPipelineTarget;
use crate::prelude::*;
use beet_rsx::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use syn::ItemFn;
// use std::path::PathBuf;
// use sweet::prelude::ReadFile;
// use syn::Visibility;



/// For a given file group, collect all public functions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFuncsToRouteTypes {}


impl Default for FileFuncsToRouteTypes {
	fn default() -> Self { Self {} }
}



impl RsxPipeline<Vec<FileFuncs>, Vec<RouteType>> for FileFuncsToRouteTypes {
	fn apply(self, value: Vec<FileFuncs>) -> Vec<RouteType> {
		value
			.into_iter()
			.map(|file_funcs| RouteType::from_file_funcs(file_funcs))
			.collect()
	}
}


pub struct RouteType {
	pub func: ItemFn,
	pub route: Route,
}

impl RsxPipelineTarget for RouteType {}

impl RouteType {
	pub fn from_file_funcs(file_funcs: FileFuncs) -> Self {
		Self {
			func: syn::parse_quote!(
				fn foobar() {}
			),
			route: file_funcs.route,
		}
	}
}
