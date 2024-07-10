#[rustfmt::skip]
pub mod expect_action{
	/// Action observers are removed when the component is so
	/// we always expect the component to exist.
	pub const ACTION_QUERY_MISSING: &str = 
		"Action entity missing from observer query";

}

pub mod expect_asset {

	pub const NOT_READY: &str = "Asset was not ready, will not run";
}
