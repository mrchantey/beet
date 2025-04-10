use crate::prelude::*;
use beet::prelude::*;





#[derive(Node)]
pub struct BeetHeaderLinks;

fn beet_header_links(_: BeetHeaderLinks) -> RsxNode {
	rsx! {
		<a class="bt-u-button-like" href=paths::docs::index()>
			Docs
		</a>
		<a class="bt-u-button-like" href=paths::index()>
			Design
		</a>
	}
}
