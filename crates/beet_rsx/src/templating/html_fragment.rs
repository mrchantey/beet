use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// Utilities for rendering HTML fragments.
pub struct HtmlFragment;

impl HtmlFragment {
	/// returns the HTML string representation of a given [`Bundle`].
	/// There are several transformations involved in the process,
	/// for example resolving slots, so we reuse a [`TemplateApp`]
	/// and run a full update cycle.
	/// The app disables loading snippets.
	pub fn parse_bundle(bundle: impl Bundle) -> String {
		// TODO bench caching and reusing the app
		let mut app = App::new();
		app.add_plugins(ApplyDirectivesPlugin);
		let entity = app.world_mut().spawn(bundle).id();
		app.update();
		let html = app
			.world_mut()
			.run_system_cached_with(render_fragment, entity)
			.unwrap();
		app.world_mut().despawn(entity);
		html
	}
}
/// A one-off system to render a single HTML fragment.
pub fn render_fragment(In(entity): In<Entity>, builder: HtmlBuilder) -> String {
	let mut str = String::new();
	builder.parse(entity, &mut str);
	str
}

/// Assign a javascript function that will collect events until hydration
pub(super) fn insert_event_playback_attribute(
	mut commands: Commands,
	html_constants: Res<HtmlConstants>,
	query: Populated<
		(&DomIdx, &Attributes),
		(With<EventTarget>, Added<DomIdx>),
	>,
	// potential event attributes
	attributes: Query<(Entity, &AttributeKey), Without<TextNode>>,
) {
	for (idx, attrs) in query.iter() {
		for (attr_entity, _) in attrs
			.iter()
			.filter_map(|attr| attributes.get(attr).ok())
			.filter(|(_, key)| key.starts_with("on"))
		{
			commands.entity(attr_entity).insert(TextNode::new(format!(
				"{}({}, event)",
				html_constants.event_handler,
				idx.inner()
			)));
		}
	}
}


// TODO bench this approach vs concatenating parallel systems
#[rustfmt::skip]
#[derive(SystemParam)]
pub struct HtmlBuilder<'w, 's> {
	fragment_nodes: Query<'w, 's,(
		&'static FragmentNode,
		&'static Children
	)>,
	doctype_nodes: Query<'w, 's, &'static DoctypeNode>,
	comment_nodes: Query<'w, 's, &'static CommentNode>,
	text_nodes: Query<'w, 's, &'static TextNode, Without<AttributeOf>>,
	element_nodes: Query<'w,'s,(
		&'static ElementNode,
		&'static NodeTag,
		Option<&'static InnerText>,
		Option<&'static Attributes>,
		Option<&'static Children>
	)>,
	attribute_nodes: Query<'w,'s,(
		&'static AttributeKey,
		Option<&'static TextNode>
	)>,
}

impl HtmlBuilder<'_, '_> {
	fn parse(&self, entity: Entity, html: &mut String) {
		if let Ok(_) = self.doctype_nodes.get(entity) {
			html.push_str("<!DOCTYPE html>");
		}
		if let Ok(comment) = self.comment_nodes.get(entity) {
			html.push_str(&format!("<!--{}-->", comment.0));
		}
		if let Ok(text) = self.text_nodes.get(entity) {
			// TODO escape html!
			html.push_str(&text);
		}
		if let Ok((_, children)) = self.fragment_nodes.get(entity) {
			for child in children.iter() {
				self.parse(child, html);
			}
		}
		if let Ok((element, tag, inner_text, attributes, children)) =
			self.element_nodes.get(entity)
		{
			let is_self_closing =
				element.self_closing && **tag != "style" && **tag != "script";

			html.push_str(&format!("<{}", tag.0));
			// add attributes
			if let Some(attrs) = attributes {
				for (key, value) in attrs
					.iter()
					.filter_map(|attr| self.attribute_nodes.get(attr).ok())
				{
					html.push(' ');
					html.push_str(&key);
					if let Some(value) = &value {
						html.push_str("=\"");
						html.push_str(&value.to_string());
						html.push_str("\"");
					}
				}
			}

			if is_self_closing {
				html.push_str("/>");
				return;
			}
			html.push('>');

			if let Some(inner_text) = inner_text {
				//TODO escape html!
				html.push_str(&inner_text.0);
			}

			if let Some(children) = children {
				for child in children.iter() {
					self.parse(child, html);
				}
			}
			html.push_str(&format!("</{}>", tag.0));
		}
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn doctype() {
		rsx! { <!DOCTYPE /> }
			.xmap(HtmlFragment::parse_bundle)
			.xpect_eq("<!DOCTYPE html>");
	}

	#[test]
	fn comment() {
		// comment (in rstml must be quoted)
		rsx! { <!-- "howdy" --> }
			.xmap(HtmlFragment::parse_bundle)
			.xpect_eq("<!--howdy-->");
	}

	#[test]
	fn raw_text() {
		rsx! { howdy }
			.xmap(HtmlFragment::parse_bundle)
			.xpect_eq("howdy");
	}

	#[test]
	fn quoted_text() {
		rsx! { "howdy" }
			.xmap(HtmlFragment::parse_bundle)
			.xpect_eq("howdy");
	}

	#[test]
	fn fragment() {
		rsx! { <>"howdy"</> }
			.xmap(HtmlFragment::parse_bundle)
			.xpect_eq("howdy");
	}

	#[test]
	fn block() {
		rsx! { {"howdy"} }
			.xmap(HtmlFragment::parse_bundle)
			.xpect_eq("howdy");
	}

	#[test]
	fn self_closing_tag() {
		rsx! { <br /> }
			.xmap(HtmlFragment::parse_bundle)
			.xpect_eq("<br/>");
	}

	#[test]
	fn not_self_closing_tag() {
		rsx! { <span>hello</span> }
			.xmap(HtmlFragment::parse_bundle)
			.xpect_eq("<span>hello</span>");
	}

	#[test]
	fn child_elements() {
		rsx! {
			<span>
				<span>hello</span>
			</span>
		}
		.xmap(HtmlFragment::parse_bundle)
		.xpect_eq("<span><span>hello</span></span>");
	}

	#[test]
	fn simple_attribute() {
		rsx! { <div class="container"></div> }
			.xmap(HtmlFragment::parse_bundle)
			.xpect_eq("<div class=\"container\"></div>");
	}

	#[test]
	fn multiple_attributes() {
		rsx! { <div class="container" id="main"></div> }
			.xmap(HtmlFragment::parse_bundle)
			.xpect_eq("<div class=\"container\" id=\"main\"></div>");
	}

	#[test]
	fn boolean_attribute() {
		rsx! { <input disabled /> }
			.xmap(HtmlFragment::parse_bundle)
			.xpect_eq("<input disabled/>");
	}

	#[test]
	fn attribute_in_self_closing_tag() {
		rsx! { <img src="/image.jpg" /> }
			.xmap(HtmlFragment::parse_bundle)
			.xpect_eq("<img src=\"/image.jpg\"/>");
	}

	#[test]
	fn complex_nested_with_attributes() {
		rsx! {
			<div class="wrapper">
				<span id="text">content</span>
			</div>
		}
		.xmap(HtmlFragment::parse_bundle)
		.xpect_eq(
			"<div class=\"wrapper\"><span id=\"text\">content</span></div>",
		);
	}

	#[test]
	fn expr_attributes() {
		let val = true;
		rsx! { <input hidden=val /> }
			.xmap(HtmlFragment::parse_bundle)
			.xpect_eq("<input hidden=\"true\"/>");
	}

	#[template]
	#[derive(Reflect)]
	fn Template() -> impl Bundle {
		rsx! {
			<div class="container">
				<span>hello</span>
			</div>
		}
	}
	#[test]
	fn templates() {
		rsx! {
			"outer"
			<Template />
		}
		.xmap(HtmlFragment::parse_bundle)
		.xpect_eq("outer<div class=\"container\"><span>hello</span></div>");
	}
	#[test]
	fn client_islands() {
		rsx! {
			"outer"
			<Template client:load />
		}
		.xmap(HtmlFragment::parse_bundle)
		.xpect_eq("outer<div class=\"container\"><span>hello</span></div>");
	}
	#[test]
	fn events() {
		rsx! { <div onclick=|| {} /> }
			.xmap(HtmlDocument::parse_bundle)
			.xpect()
			.to_contain(
				"<div onclick=\"_beet_event_handler(0, event)\" data-beet-dom-idx=\"0\"/>",
			);
	}
	#[test]
	fn iterators() {
		rsx! {
			<div>
				{vec![
					rsx! { <span>foo</span> },
					rsx! { <span>bar</span> },
					rsx! { <span>baz</span> },
				]}
			</div>
		}
		.xmap(HtmlFragment::parse_bundle)
		.xpect_eq(
			"<div><span>foo</span><span>bar</span><span>baz</span></div>",
		);
	}
	#[test]
	fn signal_text_nodes() {
		let (get, _set) = signal("foo");
		rsx! { <div>{get}</div> }
			.xmap(HtmlDocument::parse_bundle).xpect()
		.to_be_str("<!DOCTYPE html><html><head></head><body><div data-beet-dom-idx=\"0\"><!--bt|1-->foo<!--/bt--></div></body></html>");
	}

	#[test]
	#[cfg(feature = "css")]
	fn style_inline() {
		HtmlFragment::parse_bundle(rsx! {<style>body { color: red; }</style>})
			.xpect()
			.to_be_snapshot();
	}

	#[test]
	#[cfg(not(feature = "client"))]
	fn style_src() {
		HtmlFragment::parse_bundle(
			rsx! { <style src="../../tests/test_file.css" /> },
		)
		.xpect()
		.to_be_snapshot();
	}



	#[test]
	fn script() {
		HtmlFragment::parse_bundle(
			rsx! { <script type="pizza">let foo = "bar"</script> },
		)
		.xpect()
		.to_be_str("<script type=\"pizza\">let foo = \"bar\"</script>");
	}
	#[test]
	fn code() {
		HtmlFragment::parse_bundle(
			rsx! { <code lang="js">let foo = "bar"</code> },
		)
		.xpect()
		.to_be_snapshot();
	}
	#[test]
	fn escapes() {
		HtmlFragment::parse_bundle(rsx! {
			<pre>
				<code class="language-rust">fn foobar() -> String {}</code>
			</pre>
		})
		.xpect()
		.to_be_snapshot();
	}
}
