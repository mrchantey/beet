use std::cell::RefCell;

use crate::prelude::*;
use beet_common::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// returns the HTML string representation of a given [`Bundle`].
/// There are several transformations involved in the process,
/// for example resolving slots, so we reuse a [`TemplateApp`]
/// and run a full update cycle.
pub fn bundle_to_html(bundle: impl Bundle) -> String {
	/// Access the thread local [`App`] used by the [`TemplatePlugin`].
	struct SharedTemplateApp;

	impl SharedTemplateApp {
		pub fn with<O>(func: impl FnOnce(&mut App) -> O) -> O {
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

	SharedTemplateApp::with(|app| {
		let entity = app.world_mut().spawn((bundle, ToHtml)).id();
		app.update();
		let value = app
			.world_mut()
			.entity_mut(entity)
			.take::<RenderedHtml>()
			.expect("Expected RenderedHtml")
			.0;
		app.world_mut().despawn(entity);
		value
	})
}

pub fn render_html_plugin(app: &mut App) {
	app.add_systems(Update, render_html.in_set(RenderStep));
}

/// Marker indicating that the entity should be converted to HTML,
/// appending a [`RenderedHtml`] component.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct ToHtml;

#[derive(Default, Component, Deref, DerefMut, Reflect)]
#[reflect(Default, Component)]
pub struct RenderedHtml(pub String);


fn render_html(
	mut commands: Commands,
	query: Populated<Entity, Added<ToHtml>>,
	builder: Builder,
) {
	for entity in query.iter() {
		let mut html = String::new();
		builder.parse(entity, &mut html);
		commands.entity(entity).insert(RenderedHtml(html));
	}
}


// TODO bench this approach vs concatenating parallel systems 
#[rustfmt::skip]
#[derive(SystemParam)]
struct Builder<'w, 's> {
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
	// event_bindings:Query<'w,'s,(),With<SpawnedEntityObserver>>,
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
			.xmap(bundle_to_html)
			.xpect()
			.to_be("<!DOCTYPE html>");
		// comment (in rstml must be quoted)
		rsx! {<!-- "howdy" -->}
			.xmap(bundle_to_html)
			.xpect()
			.to_be("<!-- howdy -->");
		// raw text
		rsx! {howdy}.xmap(bundle_to_html).xpect().to_be("howdy");
		// quoted text
		rsx! {"howdy"}.xmap(bundle_to_html).xpect().to_be("howdy");
		// fragment
		rsx! {<>"howdy"</>}
			.xmap(bundle_to_html)
			.xpect()
			.to_be("howdy");
		// block
		rsx! {{"howdy"}}.xmap(bundle_to_html).xpect().to_be("howdy");
		// self closing
		rsx! {<br/>}.xmap(bundle_to_html).xpect().to_be("<br/>");
		// not self closing
		rsx! {<span>hello</span>}
			.xmap(bundle_to_html)
			.xpect()
			.to_be("<span>hello</span>");
		// child elements
		rsx! {<span><span>hello</span></span>}
			.xmap(bundle_to_html)
			.xpect()
			.to_be("<span><span>hello</span></span>");
		// simple attribute
		rsx! {<div class="container"></div>}
			.xmap(bundle_to_html)
			.xpect()
			.to_be("<div class=\"container\"></div>");
		// multiple attributes
		rsx! {<div class="container" id="main"></div>}
			.xmap(bundle_to_html)
			.xpect()
			.to_be("<div class=\"container\" id=\"main\"></div>");
		// boolean attribute
		rsx! {<input disabled/>}
			.xmap(bundle_to_html)
			.xpect()
			.to_be("<input disabled/>");
		// attribute in self-closing
		rsx! {<img src="image.jpg"/>}
			.xmap(bundle_to_html)
			.xpect()
			.to_be("<img src=\"image.jpg\"/>");
		// complex nested with attributes
		rsx! {<div class="wrapper"><span id="text">content</span></div>}
			.xmap(bundle_to_html)
			.xpect()
			.to_be(
				"<div class=\"wrapper\"><span id=\"text\">content</span></div>",
			);
		// expr attributes
		let val = true;
		rsx! {<input hidden=val/>}
			.xmap(bundle_to_html)
			.xpect()
			.to_be("<input hidden=\"true\"/>");
	}

	#[test]
	#[ignore = "wip"]
	fn templates() {
		#[template]
		fn Template() -> impl Bundle {
			rsx! {<div class="container"><span>hello</span></div>}
		}
		rsx! {
			"outer"
			<Template/>
		}
		.xmap(bundle_to_html)
		.xpect()
		.to_be("outer<div class=\"container\"><span>hello</span></div>");
	}
}
