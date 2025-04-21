use crate::prelude::*;


/// Wraps an entire page, including the head and body
#[derive(Node)]
pub struct DocumentLayout {
	// pub head: Head,
}

fn document_layout(_props: DocumentLayout) -> RsxNode {
	rsx! {
		<!DOCTYPE html>
		<html lang="en">
			<Head>
				<slot name="head" />
			</Head>
			<body>
				<script src="../css/initColorScheme.js" />
				<slot />
			</body>
		</html>
	}
}
