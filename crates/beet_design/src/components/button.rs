use crate::prelude::*;


/// A button component
#[derive(Node)]
pub struct Button;


impl Component for Button {
	fn render(self) -> RsxRoot {
		rsx! {
		<button>
			<slot/>
		</button>
		}
	}
}
