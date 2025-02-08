#[rustfmt::skip]
pub mod expect_action{

	/// Action observers are removed when the component is so
	/// we always expect the component to exist.
	pub const ACTION_QUERY_MISSING: &str = 
		"Action entity missing from observer query";
	pub const TARGET_MISSING: &str = 
		"`TargetEntity` does not have the components required for this action";
}
#[rustfmt::skip]
pub mod expect_run{
    use crate::events::OnRunGlobal;


	pub fn to_have_node(on_run:&OnRunGlobal) -> String {
		format!("Expected `OnRun` to specify an action, this is usually because `OnRun::new()` was triggered globally\nReceived: {:?}", on_run)
	}

	/// Action observers are removed when the component is so
	/// we always expect the component to exist.
	pub const ACTION_QUERY_MISSING: &str = 
		"Action entity missing from observer query";
	pub const TARGET_MISSING: &str = 
		"`TargetEntity` does not have the components required for this action";
}

pub mod expect_asset {

	pub const NOT_READY: &str = "Asset was not ready, will not run";
}
