use crate::prelude::*;
use beet::prelude::*;





#[template]
pub fn BeetHeaderLinks() -> impl Bundle {
	rsx! {
		<Link
			variant=ButtonVariant::Text
			href=routes::docs::index()
			>
			Docs
		</Link>
		<Link
			variant=ButtonVariant::Text
			href=routes::blog::index()
			>
			Blog
		</Link>
	}
}
