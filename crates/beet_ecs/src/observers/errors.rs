#[rustfmt::skip]
pub mod expect_action{
	/// Action observers are removed when the component is so
	/// we always expect the component to exist.
	pub const ACTION_QUERY_MISSING: &str = 
		"Action entity missing from observer query";

}
