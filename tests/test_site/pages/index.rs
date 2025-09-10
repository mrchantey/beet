use bevy::prelude::*;
use crate::as_beet::*;
use crate::test_site::*;

pub fn get() -> impl Bundle {
	// rsx! { <div>{"party time!"}</div> }
	rsx! { <PageLayout title="Test Site">party time!</PageLayout> }
}
