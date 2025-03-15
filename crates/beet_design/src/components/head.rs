use crate::prelude::*;




/// A `<head>` element with sensible defaults
#[derive(Node)]
pub struct Head {
	/// Applied to `<title>` and various `<meta>` tags
	title: String,
	///
	description: String,
}

/// foobar
fn head(props: Head) -> RsxRoot {
	rsx! {
		<head>
			<title>{props.title}</title>
			<meta name="description" content={props.description}/>
		</head>
	}
}
