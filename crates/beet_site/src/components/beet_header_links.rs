use crate::prelude::*;
use beet::prelude::*;





#[derive(Node)]
pub struct BeetHeaderLinks;

fn beet_header_links(_: BeetHeaderLinks) -> RsxNode {
	rsx! {
		<Link 
			variant=ButtonVariant::Text 
			href=paths::docs::index()
			>
			Docs
		</Link>
	}
}
