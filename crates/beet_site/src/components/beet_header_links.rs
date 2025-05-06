use crate::prelude::*;
use beet::prelude::*;





#[derive(Node)]
pub struct BeetHeaderLinks;

fn beet_header_links(_: BeetHeaderLinks) -> WebNode {
	rsx! {
		<Link 
			variant=ButtonVariant::Text 
			href=paths::docs::index()
			>
			Docs
		</Link>
	}
}
