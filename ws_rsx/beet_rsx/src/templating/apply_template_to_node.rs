use crate::prelude::*;
use beet_common::prelude::*;


/// Apply the template for the given [`WebNode`] if it has a location and
/// the location is inside the templates root directory, otherwise return Ok(()).
///
/// # Errors
/// - If

pub struct ApplyTemplateToNode;




impl Pipeline<(WebNode, WebNodeTemplate), TemplateResult<WebNode>>
	for ApplyTemplateToNode
{
	fn apply(
		self,
		(node, template): (WebNode, WebNodeTemplate),
	) -> TemplateResult<WebNode> {
		// println!("found template for node: {}\n{:?}", location, template);
		self.apply_to_node(template, &mut node.xpipe(NodeToRustyPartMap))
	}
}

impl ApplyTemplateToNode {
	/// drain the effect map into an WebNode. This does not recurse into
	/// [`RsxBlock::initial`] or [`RsxComponent::node`].
	fn apply_to_node(
		&self,
		template: WebNodeTemplate,
		rusty_map: &mut RustyPartMap,
	) -> TemplateResult<WebNode> {
		let node: WebNode = match template {
			WebNodeTemplate::Doctype { meta } => RsxDoctype { meta }.into(),
			WebNodeTemplate::Text { value, meta } => {
				RsxText { value, meta }.into()
			}
			WebNodeTemplate::Comment { value, meta } => {
				RsxComment { value, meta }.into()
			}

			WebNodeTemplate::Fragment { items, meta } => {
				let nodes = items
					.into_iter()
					.map(|item| self.apply_to_node(item, rusty_map))
					.collect::<TemplateResult<Vec<_>>>()?;
				RsxFragment { nodes, meta }.into()
			}
			WebNodeTemplate::Component {
				tracker,
				tag,
				slot_children,
				meta,
			} => {
				let (node, type_name, ron) =
					match rusty_map.remove(&tracker).ok_or_else(|| {
						TemplateError::no_rusty_map(
							&format!("Component: {}", tag),
							rusty_map.keys(),
							tracker,
						)
					})? {
						RustyPart::Component {
							root,
							type_name,
							ron,
						} => Ok((root, type_name, ron)),
						other => TemplateResult::Err(
							TemplateError::UnexpectedRusty {
								expected: "Component",
								received: format!("{:?}", other),
							},
						),
					}?;
				// very confusing to callback to the map like this
				// let root = node.xpipe(template_map)?;
				RsxComponent {
					tag,
					tracker,
					type_name,
					ron,
					// the node has no template applied yet, that is the
					// responsibility of the [`NodeTemplateMap`]
					node: Box::new(node),
					slot_children: Box::new(
						self.apply_to_node(*slot_children, rusty_map)?,
					),
					meta,
				}
				.into()
			}
			WebNodeTemplate::RustBlock { tracker, meta } => {
				let (initial, effect) =
					match rusty_map.remove(&tracker).ok_or_else(|| {
						TemplateError::no_rusty_map(
							&format!("RustBlock"),
							rusty_map.keys(),
							tracker,
						)
					})? {
						RustyPart::RustBlock { initial, effect } => {
							Ok((initial, effect))
						}
						other => TemplateResult::Err(
							TemplateError::UnexpectedRusty {
								expected: "BlockNode",
								received: format!("{:?}", other),
							},
						),
					}?;
				RsxBlock {
					// the node has no template applied yet, that is the
					// responsibility of the [`NodeTemplateMap`]
					initial: Box::new(initial),
					effect,
					meta,
				}
				.into()
			}
			WebNodeTemplate::Element {
				tag,
				self_closing,
				attributes,
				children,
				meta,
			} => RsxElement {
				tag,
				self_closing,
				attributes: attributes
					.into_iter()
					.map(|attr| template_to_attr(attr, rusty_map))
					.collect::<TemplateResult<Vec<_>>>()?,
				children: Box::new(self.apply_to_node(*children, rusty_map)?),
				meta,
			}
			.into(),
		};

		Ok(node)
	}
}

/// drain the rusty map into the template
fn template_to_attr(
	template_attr: RsxTemplateAttribute,
	rusty_map: &mut RustyPartMap,
) -> TemplateResult<RsxAttribute> {
	let rsx_attr = match template_attr {
		RsxTemplateAttribute::Key { key } => RsxAttribute::Key { key },
		RsxTemplateAttribute::KeyValue { key, value } => {
			RsxAttribute::KeyValue { key, value }
		}
		RsxTemplateAttribute::Block(tracker) => {
			let (initial, effect) = match rusty_map
				.remove(&tracker)
				.ok_or_else(|| {
					TemplateError::no_rusty_map(
						"AttributeBlock",
						rusty_map.keys(),
						tracker,
					)
				})? {
				RustyPart::AttributeBlock {
					initial,
					effect: register,
				} => Ok((initial, register)),
				other => TemplateResult::Err(TemplateError::UnexpectedRusty {
					expected: "AttributeBlock",
					received: format!("{:?}", other),
				}),
			}?;

			RsxAttribute::Block { initial, effect }
		}
		RsxTemplateAttribute::BlockValue { key, tracker } => {
			let (initial, effect) = match rusty_map
				.remove(&tracker)
				.ok_or_else(|| {
					TemplateError::no_rusty_map(
						"AttributeValue",
						rusty_map.keys(),
						tracker,
					)
				})? {
				RustyPart::AttributeValue {
					initial,
					effect: register,
				} => Ok((initial, register)),
				other => TemplateResult::Err(TemplateError::UnexpectedRusty {
					expected: "AttributeValue",
					received: format!("{:?}", other),
				}),
			}?;

			RsxAttribute::BlockValue {
				key,
				initial,
				effect,
			}
		}
	};
	Ok(rsx_attr)
}
