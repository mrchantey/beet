use crate::prelude::*;


/// Wraps an entire page, including the head and body
#[template]
pub fn DocumentLayout() -> impl Bundle {
	rsx! {
		<!DOCTYPE html>
		<html lang="en">
			<Head>
				<slot name="head" />
			</Head>
			<body>
				<slot />
			</body>
		</html>
	}
}
