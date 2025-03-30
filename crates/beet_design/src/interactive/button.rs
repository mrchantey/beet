use crate::prelude::*;


/// A button component
#[derive(Node)]
pub struct Button;

fn button(_: Button) -> RsxNode {
	rsx! {
	<button>
		<slot/>
	</button>
	}
}
