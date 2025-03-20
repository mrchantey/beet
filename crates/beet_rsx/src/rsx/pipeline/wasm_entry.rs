#![allow(unused)]
use crate::prelude::*;





/// Find the first `client:load` directive, if any, and return that component.
#[derive(Default)]
pub struct WasmEntry;


impl<T: RsxPipelineTarget + AsRef<RsxRoot>> RsxPipeline<T, Option<RsxComponent>>
	for WasmEntry
{
	fn apply(self, root: T) -> Option<RsxComponent> {
		let mut result = None;

		VisitRsxComponent::walk(root.as_ref(), |component| {

			// if component.root.node.
		});

		result
	}
}
