



use beet_rsx::as_beet::*;

#[derive(Node)]
pub struct TextField{
	pub field: String,
}

fn text_field(props: TextField) -> RsxNode {
	
	
	rsx!{
		<div>{props.field}</div>
	}
}