use beet::prelude::*;

/// Demonstrates a [`Form`] composed of a [`TextField`] and a submit [`Button`].
///
/// The legacy live `DynamicStruct` submit demo is dropped; this page shows the
/// static form layout that renders across web and terminal.
pub fn get() -> impl Scene {
	rsx! {
		<article>
			<h1>"Form"</h1>
			<Form>
				<label>"Name"</label>
				<TextField name="name" placeholder="Ada Lovelace"/>
				<Button label="Submit" variant=ButtonVariant::Filled/>
			</Form>
		</article>
	}
}
