use crate::prelude::*;


/// A button component
#[derive(Node)]
pub struct Button;

fn button(_: Button) -> RsxRoot {
	rsx! {
	<button>
		<slot/>
	</button>
	}
}
