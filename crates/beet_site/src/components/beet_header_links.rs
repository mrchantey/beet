use crate::prelude::*;
use beet::prelude::*;





#[template]
pub fn BeetHeaderLinks() -> impl Bundle {
	rsx! {
		<Link
			variant=ButtonVariant::Text
			href=paths::docs::index()
			>
			Docs
		</Link>
	}
}
