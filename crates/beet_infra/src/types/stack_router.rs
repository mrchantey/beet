use beet_core::prelude::*;
use beet_router::prelude::*;



pub fn stack_router() -> impl Bundle {
	(
		default_router(),
		OnSpawn::insert_child(route_tool("validate", Validate)),
	)
}


#[tool]
fn Validate() -> String { "VALIDATED".into() }
