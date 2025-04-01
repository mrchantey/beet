use crate::prelude::*;
use beet::prelude::*;


pub fn get() -> RsxNode {
	rsx! {
		<BeetPage>
			<div>party time dude!</div>
		</BeetPage>
	}
}
