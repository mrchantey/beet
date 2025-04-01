use crate::prelude::*;


pub enum ButtonVariant {
	Primary,
	Secondary,
	Tertiary,
	// Destructive,
	// Link,
}


/// A button component
#[derive(Node)]
pub struct Button {
	#[field(default=ButtonVariant::Primary)]
	pub variant: ButtonVariant,
}

fn button(_: Button) -> RsxNode {
	rsx! {
		<button>
			<slot />
		</button>
	}
}
