// use crate::prelude::*;
use beet::prelude::*;





#[derive(Node)]
pub struct BeetHeaderLinks;

fn beet_header_links(_: BeetHeaderLinks) -> RsxNode {
	rsx! { <div>over here!</div> }
}
