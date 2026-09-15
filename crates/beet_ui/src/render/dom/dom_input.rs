//! The DOM input path: the document's events, delivered to the world as the
//! renderer-agnostic events the terminal bridge emits.
use super::*;
use crate::prelude::*;
use beet_core::exports::SendWrapper;
use beet_core::prelude::*;
use wasm_bindgen::JsCast;

/// Delivers the browser's input to the world through the events the widgets
/// already react to, so a page in the browser behaves as the same page does on
/// the terminal.
///
/// A mount root (an entity [`DomRenderer::mount`] bound to a document element
/// it did not paint, the twin of a terminal surface) installs one set of
/// delegated listeners on that element ([`DomListeners`]), and every frame
/// drains what they queued, in the order it fired:
///
/// - `click` is the activation: [`PointerDown`] then [`PointerUp`] on the
///   target's entity, the press and release the terminal's hit test fires. The
///   composed event rather than the pointer pair, so a native control the
///   keyboard activates (Enter on a `<button>`, Space on a checkbox) and a
///   programmatic `click()` reach the world exactly as a mouse does.
/// - `pointerover`/`pointerout` are [`PointerOver`]/[`PointerOut`].
/// - `input`/`change` on a control write its live value into the entity's
///   [`Value`], the direct write typing makes on the terminal, no change
///   flagged when the world already holds it. Native controls own text, caret
///   and IME, so no key ever reaches the world as text.
/// - `focusin`/`focusout` insert and remove [`Focus`]: the world mirrors the
///   document's focus rather than driving it.
/// - Enter or Space on a focusable with no native activation (a tree row) is
///   the pointer pair, the twin of the terminal's Enter activation; its default
///   (Space scrolling the page) is prevented.
/// - `submit` is prevented at dispatch, and lands [`Submit`] with the form's
///   world values when no button submitted it (Enter in a lone field); a
///   submitting button's own activation has already fired it through the click.
///
/// A mount root also takes what the page did before the world existed: the
/// element the document had focused takes [`Focus`], and the `click`s and
/// `submit`s the page's pre-boot script queued ([`PreBoot`]) are dispatched
/// through the same table against the nodes the mount just bound.
///
/// Composes the focus model, the hover state and the form behaviour the
/// terminal composes, so the same observers serve both.
#[derive(Default)]
pub struct DomInputPlugin;

impl Plugin for DomInputPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<FocusPlugin>()
			.init_plugin::<PointerStatePlugin>()
			.add_observer(listen_on_mount)
			// the frame's first thing, so a control's write reaches its document
			// in this frame's sync pass rather than the next
			.add_systems(First, drain_dom_input);
		#[cfg(feature = "template")]
		app.init_plugin::<FormPlugin>();
	}
}

/// The delegated listeners a mount root installs on its document element: one
/// queue for every event kind, so a frame drains them in the order they fired
/// (a checkbox's activation before the `change` reporting its new state).
#[derive(Component)]
pub(crate) struct DomListeners(SendWrapper<HtmlEventListener>);

impl DomListeners {
	/// Listen on `target` for everything the world maps.
	fn listen(target: web_sys::Element) -> Self {
		HtmlEventListener::queue()
			.listen("click", target.clone())
			.listen("pointerover", target.clone())
			.listen("pointerout", target.clone())
			.listen("input", target.clone())
			.listen("change", target.clone())
			.listen("focusin", target.clone())
			.listen("focusout", target.clone())
			.listen_filtered(
				"keydown",
				target.clone(),
				activates_without_native,
			)
			.listen_filtered("submit", target, |ev| {
				// the world serves the form; the page never navigates
				ev.prevent_default();
				true
			})
			.xmap(SendWrapper::new)
			.xmap(Self)
	}

	/// The next queued event, if any.
	fn try_next(&mut self) -> Option<web_sys::Event> { self.0.try_next_event() }
}

/// Tags whose activation the browser owns: Enter or Space on one clicks,
/// toggles, opens or types natively, so the world synthesizes nothing.
const NATIVE_ACTIVATION: &[&str] = &[
	"a", "button", "details", "input", "option", "select", "summary",
	"textarea",
];

/// Whether a key event is Enter or Space on a focusable the browser does not
/// activate itself, the one case the world activates; its default is prevented
/// so Space never scrolls the page under a row.
fn activates_without_native(ev: &web_sys::Event) -> bool {
	let Some(key) = ev.dyn_ref::<web_sys::KeyboardEvent>() else {
		return false;
	};
	if !matches!(key.key().as_str(), "Enter" | " ") {
		return false;
	}
	let native = ev
		.target()
		.and_then(|target| target.dyn_into::<web_sys::Element>().ok())
		.is_none_or(|element| {
			NATIVE_ACTIVATION
				.contains(&element.tag_name().to_lowercase().as_str())
		});
	if native {
		return false;
	}
	ev.prevent_default();
	true
}

/// Observer: a mount root, bound to a document element it did not paint (a
/// `<body>` entity paints as its own), listens for the page's input there,
/// then takes what happened before it could: the document's focus, and the
/// page's pre-boot queue.
fn listen_on_mount(
	ev: On<Insert, DomNode>,
	roots: Query<&DomNode, (Without<Element>, Without<DomListeners>)>,
	mut commands: Commands,
) {
	let Ok(DomNode::Document(element)) = roots.get(ev.entity) else {
		return;
	};
	let root = ev.entity;
	commands
		.entity(root)
		.insert(DomListeners::listen((**element).clone()));
	commands.queue(move |world: &mut World| replay_pre_boot(world, root));
}

/// What the page did before the world existed, now that the mount has bound
/// its nodes: the focused element's entity takes [`Focus`], since its
/// `focusin` fired with nobody listening, and every queued `click` and
/// `submit` ([`PreBoot`]) is dispatched as if it had just fired. A click on
/// a control that toggles on click is not replayed: its `checked` already
/// changed, and adoption read it.
fn replay_pre_boot(world: &mut World, root: Entity) {
	if let Some(focused) = document_ext::document()
		.active_element()
		.and_then(|element| DomNode::entity_of(&element))
		&& world.get_entity(focused).is_ok()
	{
		world.entity_mut(focused).insert(Focus);
	}
	for ev in PreBoot::take_queue() {
		let toggled = ev.type_() == "click"
			&& ev
				.target()
				.and_then(|target| {
					target.dyn_into::<web_sys::HtmlInputElement>().ok()
				})
				.is_some_and(|input| LiveValue::toggles_on_click(&input));
		if !toggled {
			dispatch(world, root, &ev);
		}
	}
}

/// System: every event the mount roots queued since the last frame, dispatched
/// in order. Exclusive, so each lands before the next is read: the activation
/// a checkbox's `change` follows has toggled the world by the time the change
/// writes what the document holds.
fn drain_dom_input(world: &mut World) {
	let roots: Vec<Entity> = world
		.query_filtered::<Entity, With<DomListeners>>()
		.iter(world)
		.collect();
	for root in roots {
		while let Some(ev) = world
			.get_mut::<DomListeners>(root)
			.and_then(|mut listeners| listeners.try_next())
		{
			dispatch(world, root, &ev);
		}
	}
}

/// One event onto the entity its target paints: the mount root stands in as
/// the pointer, as the terminal surface does.
fn dispatch(world: &mut World, root: Entity, ev: &web_sys::Event) {
	let Some(target) = ev
		.target()
		.and_then(|target| target.dyn_into::<web_sys::Node>().ok())
		.and_then(|node| DomNode::entity_of(&node))
		.filter(|target| world.get_entity(*target).is_ok())
	else {
		return;
	};
	match ev.type_().as_str() {
		"click" | "keydown" => activate(world, root, target),
		"pointerover" => {
			world.entity_mut(target).trigger(PointerOver::new(root));
		}
		"pointerout" => {
			world.entity_mut(target).trigger(PointerOut::new(root));
		}
		"input" | "change" => write_control_value(world, target, ev),
		"focusin" => {
			world.entity_mut(target).insert(Focus);
		}
		"focusout" => {
			world.entity_mut(target).remove::<Focus>();
		}
		#[cfg(feature = "template")]
		"submit" => submit(world, target, ev),
		_ => {}
	}
}

/// The pointer pair on `target`: a click, or Enter/Space on a focusable the
/// browser does not activate.
fn activate(world: &mut World, root: Entity, target: Entity) {
	world.entity_mut(target).trigger(PointerDown::new(root));
	// the press may have taken the target with it
	if world.get_entity(target).is_ok() {
		world.entity_mut(target).trigger(PointerUp::new(root));
	}
}

/// An `input`/`change`: the control's live value into its entity's [`Value`].
fn write_control_value(world: &mut World, target: Entity, ev: &web_sys::Event) {
	if let Some(live) = ev.target().and_then(|target| LiveValue::read(&target))
	{
		live.write(world, target);
	}
}

/// A `submit`, its navigation already prevented at dispatch: [`Submit`] with
/// the form's world values when nothing submitted it but the form itself
/// (Enter in a lone field, a `requestSubmit()`); a submitting button's click
/// has fired it already.
#[cfg(feature = "template")]
fn submit(world: &mut World, form: Entity, ev: &web_sys::Event) {
	let submitted_by_button = ev
		.dyn_ref::<web_sys::SubmitEvent>()
		.and_then(|ev| ev.submitter())
		.is_some();
	if submitted_by_button {
		return;
	}
	world
		.run_system_cached_with(
			|form: In<Entity>,
			 elements: ElementQuery,
			 values: Query<&Value>,
			 mut commands: Commands| {
				crate::widgets::trigger_form_submit(
					*form,
					&elements,
					&values,
					&mut commands,
				);
			},
			form,
		)
		.ok();
}

#[cfg(test)]
mod test {
	use super::super::test_ext::*;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use wasm_bindgen::JsCast;

	/// The pointer events an entity observer saw: the kind, the original
	/// target (the event's `target` follows the propagation) and the pointer.
	#[derive(Default, Resource)]
	struct Hits(Vec<(&'static str, Entity, Entity)>);

	/// Record the pointer pair reaching `entity`.
	fn record_hits(app: &mut App, entity: Entity) {
		app.init_resource::<Hits>();
		app.world_mut()
			.entity_mut(entity)
			.observe(|ev: On<PointerDown>, mut hits: ResMut<Hits>| {
				hits.0
					.push(("down", ev.original_event_target(), ev.pointer));
			})
			.observe(|ev: On<PointerUp>, mut hits: ResMut<Hits>| {
				hits.0.push(("up", ev.original_event_target(), ev.pointer));
			});
	}

	fn hits(app: &App) -> Vec<(&'static str, Entity, Entity)> {
		app.world().resource::<Hits>().0.clone()
	}

	/// A click is the pointer pair on the entity under it, propagating to
	/// the button an observer sits on, the mount root standing in as the
	/// pointer.
	#[beet_core::test(browser)]
	fn a_click_reaches_pointer_up_through_a_nested_target() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! { <button><span>"go"</span></button> });
		paint(&mut app, host);
		let button = element(app.world_mut(), "button");
		let span = element(app.world_mut(), "span");
		record_hits(&mut app, button);
		html_element(app.world(), span).click();
		app.update();
		hits(&app).xpect_eq(vec![("down", span, host), ("up", span, host)]);
	}

	/// Typing into a control writes its value into the world, the document
	/// takes it in the same frame's sync, and the bound text follows on the
	/// next pass while the control itself, already holding the text, keeps
	/// its caret.
	#[beet_core::test(browser)]
	fn input_writes_the_value_and_the_bound_text_repaints() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! {
			<TextField field={FieldRef::new("name")}/>
			<p>{(Value::default(), FieldRef::new("name"))}</p>
		});
		set_document(&mut app, host, value!({ "name": "pete" }));
		app.update();
		let target = paint(&mut app, host);
		let input = element(app.world_mut(), "input");
		let input = html_element(app.world(), input)
			.dyn_into::<web_sys::HtmlInputElement>()
			.unwrap();
		// the user typed: the control holds the text before its event fires
		input.set_value("peter");
		input.set_selection_range(5, 5).unwrap();
		fire(&input, &bubbling("input"));
		app.update();
		field(app.world(), host, "name").xpect_eq(Value::str("peter"));
		app.update();
		target
			.query_selector("p")
			.unwrap()
			.unwrap()
			.text_content()
			.unwrap()
			.xpect_eq("peter");
		input.selection_start().unwrap().xpect_eq(Some(5));
	}

	/// A number field keeps its kind: a parseable edit lands as a number, an
	/// unparseable one (mid-edit) leaves the world's number alone.
	#[beet_core::test(browser)]
	fn a_number_field_keeps_its_kind() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! {
			<NumberField field={FieldRef::new("count")}/>
		});
		set_document(&mut app, host, value!({ "count": 5 }));
		app.update();
		paint(&mut app, host);
		let input = element(app.world_mut(), "input");
		let input = html_element(app.world(), input)
			.dyn_into::<web_sys::HtmlInputElement>()
			.unwrap();
		input.set_value("53");
		fire(&input, &bubbling("input"));
		app.update();
		field(app.world(), host, "count").xpect_eq(Value::Int(53));
		// a browser reports an unparseable number field as empty
		input.set_value("");
		fire(&input, &bubbling("input"));
		app.update();
		field(app.world(), host, "count").xpect_eq(Value::Int(53));
	}

	/// A checkbox click toggles once: the activation flips the world, and
	/// the `change` that follows reports the state the world already holds.
	#[beet_core::test(browser)]
	fn a_checkbox_click_toggles_once() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! {
			<Checkbox field={FieldRef::new("done")}/>
		});
		set_document(&mut app, host, value!({ "done": false }));
		app.update();
		app.update();
		paint(&mut app, host);
		let checkbox = element(app.world_mut(), "input");
		let input = html_element(app.world(), checkbox)
			.dyn_into::<web_sys::HtmlInputElement>()
			.unwrap();
		input.click();
		app.update();
		input.checked().xpect_true();
		field(app.world(), host, "done").xpect_eq(Value::Bool(true));
		input.click();
		app.update();
		input.checked().xpect_false();
		field(app.world(), host, "done").xpect_eq(Value::Bool(false));
	}

	/// The world mirrors the document's focus: focusing a row inserts
	/// [`Focus`], blurring it removes it.
	#[beet_core::test(browser)]
	fn focusin_inserts_focus_and_focusout_removes_it() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! {
			<div tabindex="0">"row"</div>
			<TextField field={FieldRef::new("name")}/>
		});
		paint(&mut app, host);
		let row = element(app.world_mut(), "div");
		let input = element(app.world_mut(), "input");
		let row_node = html_element(app.world(), row);
		row_node.focus().unwrap();
		app.update();
		app.world().entity(row).contains::<Focus>().xpect_true();
		// focus moving on: the row loses it, the field takes it
		html_element(app.world(), input).focus().unwrap();
		app.update();
		app.world().entity(row).contains::<Focus>().xpect_false();
		app.world().entity(input).contains::<Focus>().xpect_true();
		html_element(app.world(), input).blur().unwrap();
		app.update();
		app.world().entity(input).contains::<Focus>().xpect_false();
	}

	/// Enter or Space on a focusable the browser does not activate fires the
	/// pointer pair and is prevented; a native control keeps its keys, and
	/// any other key is left alone.
	#[beet_core::test(browser)]
	fn enter_on_a_focused_row_fires_the_pointer_pair() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! {
			<div tabindex="0">"row"</div>
			<button>"go"</button>
		});
		paint(&mut app, host);
		let row = element(app.world_mut(), "div");
		let button = element(app.world_mut(), "button");
		record_hits(&mut app, row);
		record_hits(&mut app, button);
		let row_node = html_element(app.world(), row);
		fire(&row_node, &keydown("Enter")).xpect_false();
		fire(&row_node, &keydown(" ")).xpect_false();
		// the browser clicks a button on Enter itself
		fire(&html_element(app.world(), button), &keydown("Enter"))
			.xpect_true();
		fire(&row_node, &keydown("a")).xpect_true();
		app.update();
		hits(&app).xpect_eq(vec![
			("down", row, host),
			("up", row, host),
			("down", row, host),
			("up", row, host),
		]);
	}

	/// Hovering marks the element and its ancestors, leaving clears them.
	#[beet_core::test(browser)]
	fn pointer_over_marks_the_hovered_chain() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! { <div><span>"x"</span></div> });
		paint(&mut app, host);
		let div = element(app.world_mut(), "div");
		let span = element(app.world_mut(), "span");
		let hovered = |app: &App, entity: Entity| {
			app.world()
				.get::<ElementStateMap>(entity)
				.is_some_and(|states| states.contains(&ElementState::Hovered))
		};
		fire(&html_element(app.world(), span), &bubbling("pointerover"));
		app.update();
		hovered(&app, span).xpect_true();
		hovered(&app, div).xpect_true();
		fire(&html_element(app.world(), span), &bubbling("pointerout"));
		app.update();
		hovered(&app, span).xpect_false();
		hovered(&app, div).xpect_false();
	}

	/// A click before the world existed replays onto the entity the mount
	/// bound its target to, through the same table; a checkbox the browser
	/// ticked itself is read rather than replayed, so it toggles once; and
	/// the queue is gone once the world's own listeners have taken over.
	#[beet_core::test(browser)]
	fn a_pre_boot_click_replays_after_adoption() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! {
			<button><span>"go"</span></button>
			<Checkbox field={FieldRef::new("done")}/>
		});
		set_document(&mut app, host, value!({ "done": false }));
		app.update();
		app.update();
		let target = serve(&mut app, host);
		install_pre_boot();
		let span = target
			.query_selector("span")
			.unwrap()
			.unwrap()
			.dyn_into::<web_sys::HtmlElement>()
			.unwrap();
		let checkbox = target
			.query_selector("input")
			.unwrap()
			.unwrap()
			.dyn_into::<web_sys::HtmlInputElement>()
			.unwrap();
		span.click();
		checkbox.click();
		checkbox.checked().xpect_true();
		let button = element(app.world_mut(), "button");
		record_hits(&mut app, button);
		adopt(&mut app, host, &target);
		let span_entity = element(app.world_mut(), "span");
		hits(&app).xpect_eq(vec![
			("down", span_entity, host),
			("up", span_entity, host),
		]);
		field(app.world(), host, "done").xpect_eq(Value::Bool(true));
		checkbox.checked().xpect_true();
		js_sys::Reflect::has(&js_sys::global(), &PreBoot::GLOBAL.into())
			.unwrap()
			.xpect_false();
		// the world's listeners from here: one click, one pair
		span.click();
		app.update();
		hits(&app).len().xpect_eq(4);
	}

	/// The element the document focused before the world existed carries
	/// [`Focus`] once the mount has bound it.
	#[beet_core::test(browser)]
	fn a_pre_boot_focus_is_mirrored() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! { <div tabindex="0">"row"</div> });
		let target = serve(&mut app, host);
		let row = target
			.query_selector("div")
			.unwrap()
			.unwrap()
			.dyn_into::<web_sys::HtmlElement>()
			.unwrap();
		row.focus().unwrap();
		adopt(&mut app, host, &target);
		let row_entity = element(app.world_mut(), "div");
		app.world()
			.entity(row_entity)
			.contains::<Focus>()
			.xpect_true();
		row.blur().unwrap();
	}

	/// The [`Submit`]s an observer saw, each with the form's values.
	#[derive(Default, Resource)]
	struct Submits(Vec<Value>);

	/// A submit is prevented and lands [`Submit`] with the form's world
	/// values when the form submitted itself; a button's click has already
	/// fired it, so its submit adds nothing, and the page never navigates.
	#[beet_core::test(browser)]
	fn submit_is_prevented_and_lands_submit_with_the_forms_values() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! {
			<Form>
				<TextField name="who" field={FieldRef::new("who")}/>
				<Checkbox name="done" field={FieldRef::new("done")}/>
				<Button>"Send"</Button>
			</Form>
		});
		set_document(&mut app, host, value!({ "who": "pete", "done": true }));
		app.update();
		app.update();
		paint(&mut app, host);
		app.init_resource::<Submits>();
		app.world_mut().add_observer(
			|ev: On<Submit>, mut submits: ResMut<Submits>| {
				submits.0.push(ev.values.clone());
			},
		);
		let form = element(app.world_mut(), "form");
		let form = html_element(app.world(), form);
		let button = element(app.world_mut(), "button");
		let button = html_element(app.world(), button);
		// the form submitted itself: prevented, served by the world
		fire(&form, &submit(None)).xpect_false();
		app.update();
		app.world()
			.resource::<Submits>()
			.0
			.clone()
			.xpect_eq(vec![value!({ "who": "pete", "done": true })]);
		// a button's submit names it: the click path fired the event already
		fire(&form, &submit(Some(&button))).xpect_false();
		app.update();
		app.world().resource::<Submits>().0.len().xpect_eq(1);
		// the real thing: one click, one submit, the test page still here
		button.click();
		app.update();
		app.world().resource::<Submits>().0.len().xpect_eq(2);
	}

	/// A submit before the world existed is prevented by the pre-boot script
	/// and lands [`Submit`] on replay, the page still here.
	#[beet_core::test(browser)]
	fn a_pre_boot_submit_is_prevented_and_replayed() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! {
			<Form>
				<TextField name="who" field={FieldRef::new("who")}/>
			</Form>
		});
		set_document(&mut app, host, value!({ "who": "pete" }));
		app.update();
		let target = serve(&mut app, host);
		install_pre_boot();
		let form = target.query_selector("form").unwrap().unwrap();
		fire(&form, &submit(None)).xpect_false();
		app.init_resource::<Submits>();
		app.world_mut().add_observer(
			|ev: On<Submit>, mut submits: ResMut<Submits>| {
				submits.0.push(ev.values.clone());
			},
		);
		adopt(&mut app, host, &target);
		app.world()
			.resource::<Submits>()
			.0
			.clone()
			.xpect_eq(vec![value!({ "who": "pete" })]);
	}
}
