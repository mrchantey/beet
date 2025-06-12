use std::cell::RefCell;

use crate::prelude::*;
use beet_common::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// Add this component to have it populated with HTML on the next update cycle.
/// Html rendered from a bundle as-is without resolving any [`HtmlInsertDirective`]
/// or inserting missing body, head and doctype tags.
#[derive(Default, Component, Deref, DerefMut, Reflect)]
#[reflect(Default, Component)]
pub struct HtmlFragment(pub String);

impl HtmlFragment {
	/// returns the HTML string representation of a given [`Bundle`].
	/// There are several transformations involved in the process,
	/// for example resolving slots, so we reuse a [`TemplateApp`]
	/// and run a full update cycle.
	pub fn parse_bundle(bundle: impl Bundle) -> String {
		SharedTemplateApp::with(|app| {
			let entity = app
				.world_mut()
				.spawn((bundle, HtmlFragment::default()))
				.id();
			app.update();
			let value = app
				.world_mut()
				.entity_mut(entity)
				.take::<HtmlFragment>()
				.unwrap()
				.0;
			app.world_mut().despawn(entity);
			value
		})
	}
}
/// A thread-local [`App`] cached so a new app doesn't need to be
/// created for each render job.
pub(super) struct SharedTemplateApp;

impl SharedTemplateApp {
	#[allow(unused)]
	fn with_new<O>(func: impl FnOnce(&mut App) -> O) -> O {
		let mut app = App::new();
		app.add_plugins(TemplatePlugin);
		func(&mut app)
	}
	/// Access the thread local [`App`] used by the [`TemplatePlugin`].
	pub(super) fn with<O>(func: impl FnOnce(&mut App) -> O) -> O {
		thread_local! {
			static TEMPLATE_APP: RefCell<Option<App>> = RefCell::new(None);
		}
		TEMPLATE_APP.with(|app_cell| {
			// Initialize the app if needed
			let mut app_ref = app_cell.borrow_mut();
			if app_ref.is_none() {
				let mut app = App::new();
				app.add_plugins(TemplatePlugin);
				*app_ref = Some(app);
			}

			// Now we can safely unwrap and use the app
			let app = app_ref.as_mut().unwrap();

			func(app)
		})
	}
}

pub(super) fn render_html_fragments(
	mut query: Populated<(Entity, &mut HtmlFragment), Added<HtmlFragment>>,
	builder: Builder,
) {
	for (entity, mut html) in query.iter_mut() {
		builder.parse(entity, &mut html);
	}
}


// TODO bench this approach vs concatenating parallel systems
#[rustfmt::skip]
#[derive(SystemParam)]
pub(super) struct Builder<'w, 's> {
	elements: Query<'w,'s,(
		&'static ElementNode,
		&'static NodeTag,
		Option<&'static Attributes>,
		Option<&'static Children>
	)>,
	fragments: Query<'w, 's,(
		&'static FragmentNode,
		&'static Children
	)>,
	attributes: Query<'w,'s,(
		&'static AttributeKeyStr,
		Option<&'static AttributeValueStr>
	)>,
	doctypes: Query<'w, 's, &'static DoctypeNode>,
	comments: Query<'w, 's, &'static CommentNode>,
	texts: Query<'w, 's, &'static TextNode>,
}

impl Builder<'_, '_> {
	fn parse(&self, entity: Entity, html: &mut String) {
		if let Ok(_) = self.doctypes.get(entity) {
			html.push_str("<!DOCTYPE html>");
		}
		if let Ok(comment) = self.comments.get(entity) {
			html.push_str(&format!("<!-- {} -->", comment.0));
		}
		if let Ok(text) = self.texts.get(entity) {
			html.push_str(&text.0);
		}
		if let Ok((_, children)) = self.fragments.get(entity) {
			for child in children.iter() {
				self.parse(child, html);
			}
		}
		if let Ok((element, tag, attributes, children)) =
			self.elements.get(entity)
		{
			html.push_str(&format!("<{}", tag.0));
			// add attributes
			if let Some(attrs) = attributes {
				for (key, value) in attrs
					.iter()
					.filter_map(|attr| self.attributes.get(attr).ok())
				{
					html.push(' ');
					html.push_str(&key);
					if let Some(value) = &value {
						html.push_str("=\"");
						html.push_str(value);
						html.push_str("\"");
					}
				}
			}
			// add binding ids



			if element.self_closing {
				html.push_str("/>");
				return;
			} else {
				html.push('>');
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
	fn works() {
		// doctype
		rsx! {<!doctype/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<!DOCTYPE html>");
		// comment (in rstml must be quoted)
		rsx! {<!-- "howdy" -->}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<!-- howdy -->");
		// raw text
		rsx! {howdy}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("howdy");
		// quoted text
		rsx! {"howdy"}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("howdy");
		// fragment
		rsx! {<>"howdy"</>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("howdy");
		// block
		rsx! {{"howdy"}}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("howdy");
		// self closing
		rsx! {<br/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<br/>");
		// not self closing
		rsx! {<span>hello</span>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<span>hello</span>");
		// child elements
		rsx! {<span><span>hello</span></span>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<span><span>hello</span></span>");
		// simple attribute
		rsx! {<div class="container"></div>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<div class=\"container\"></div>");
		// multiple attributes
		rsx! {<div class="container" id="main"></div>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<div class=\"container\" id=\"main\"></div>");
		// boolean attribute
		rsx! {<input disabled/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<input disabled/>");
		// attribute in self-closing
		rsx! {<img src="image.jpg"/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<img src=\"image.jpg\"/>");
		// complex nested with attributes
		rsx! {<div class="wrapper"><span id="text">content</span></div>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be(
				"<div class=\"wrapper\"><span id=\"text\">content</span></div>",
			);
		// expr attributes
		let val = true;
		rsx! {<input hidden=val/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<input hidden=\"true\"/>");
	}

	#[test]
	fn templates() {
		#[template]
		fn Template() -> impl Bundle {
			rsx! {<div class="container"><span>hello</span></div>}
		}
		rsx! {
			"outer"
			<Template/>
		}
		.xmap(HtmlFragment::parse_bundle)
		.xpect()
		.to_be("outer<div class=\"container\"><span>hello</span></div>");
	}
	#[test]
	fn events() {
		rsx! {<div onclick={||{}}/>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<div data-beet-rsx-idx=\"0\"/>");
	}
	#[test]
	fn signal_text_nodes() {
		let (get, _set) = signal("foo");
		rsx! {<div>{get}</div>}
			.xmap(HtmlFragment::parse_bundle)
			.xpect()
			.to_be("<div data-beet-rsx-idx=\"0\">foo</div>");
	}
}
