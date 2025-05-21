#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	fn simple() {
		rsx_template! { <div>{value}</div> }
			.reset_spans_and_trackers()
			.xpect()
			.to_be(WebNodeTemplate::Element {
				tag: "div".to_string(),
				self_closing: false,
				attributes: vec![],
				meta: NodeMeta::new(FileSpan::default(), vec![
					TemplateDirectiveEnum::NodeTemplate,
				]),
				children: Box::new(WebNodeTemplate::RustBlock {
					tracker: RustyTracker::PLACEHOLDER,
					meta: NodeMeta::default(),
				}),
			});
	}
	#[test]
	fn complex() {
		let template = rsx_template! {
			<div key str="value" num=32 ident=some_val>
				<p>
					hello <MyComponent>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		}
		.reset_spans_and_trackers();

		expect(&template).to_be(&WebNodeTemplate::Element {
			tag: "div".to_string(),
			self_closing: false,
			meta: NodeMeta::new(FileSpan::default(), vec![
				TemplateDirectiveEnum::NodeTemplate,
			]),
			attributes: vec![
				RsxTemplateAttribute::Key {
					key: "key".to_string(),
				},
				RsxTemplateAttribute::KeyValue {
					key: "str".to_string(),
					value: "value".to_string(),
				},
				RsxTemplateAttribute::KeyValue {
					key: "num".to_string(),
					value: "32".to_string(),
				},
				RsxTemplateAttribute::BlockValue {
					key: "ident".to_string(),
					tracker: RustyTracker::PLACEHOLDER,
				},
			],
			children: Box::new(WebNodeTemplate::Element {
				tag: "p".to_string(),
				self_closing: false,
				attributes: vec![],
				meta: NodeMeta::default(),
				children: Box::new(WebNodeTemplate::Fragment {
					meta: NodeMeta::default(),
					items: vec![
						WebNodeTemplate::Text {
							meta: NodeMeta::default(),
							value: "\n\t\t\t\t\thello ".to_string(),
						},
						WebNodeTemplate::Component {
							meta: NodeMeta::default(),
							tracker: RustyTracker::PLACEHOLDER,
							tag: "MyComponent".to_string(),
							slot_children: Box::new(WebNodeTemplate::Element {
								tag: "div".to_string(),
								self_closing: false,
								attributes: vec![],
								meta: NodeMeta::default(),
								children: Box::new(WebNodeTemplate::Text {
									value: "some child".to_string(),
									meta: NodeMeta::default(),
								}),
							}),
						},
					],
				}),
			}),
		});
	}

	#[test]
	fn ron() {
		let template = rsx_template! {
			<div key str="value" num=32 ident=some_val>
				<p>
					hello <MyComponent>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		}
		.reset_spans_and_trackers();
		let template2 = rsx_template! {
			<div key str="value" num=32 ident=some_val>
				<p>
					hello <MyComponent>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		}
		.reset_spans_and_trackers();
		expect(template).to_be(template2);
	}
}
