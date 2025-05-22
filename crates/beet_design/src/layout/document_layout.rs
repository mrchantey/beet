use crate::prelude::*;


/// Wraps an entire page, including the head and body
#[derive(derive_template)]
pub struct DocumentLayout {
	// pub head: Head,
}

fn document_layout(_props: DocumentLayout) -> WebNode {
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
