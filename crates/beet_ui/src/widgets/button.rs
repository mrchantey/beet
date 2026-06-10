//! Button widgets: `Button`, `IconButton`, `Link`.
//!
//! Each emits the semantic [`classes::BTN`] base plus a variant class; the
//! active rule set (Material Design 3 today) maps those to design tokens. A
//! [`Link`] is an `<a>` styled as a button — the rules match any element, so a
//! hyperlink picks up the same look as a `<button>`.
use crate::prelude::*;
use beet_core::prelude::*;

/// Visual emphasis variant, mapped one-to-one onto a semantic class.
#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub enum ButtonVariant {
	/// High-emphasis primary action (`btn-filled`).
	#[default]
	Filled,
	/// Medium emphasis with a visible border (`btn-outlined`).
	Outlined,
	/// Lowest emphasis, no container (`btn-text`).
	Text,
	/// Medium emphasis using the secondary container (`btn-tonal`).
	Tonal,
	/// Medium emphasis with a shadow (`btn-elevated`).
	Elevated,
	/// Filled using the secondary color (`btn-secondary`).
	Secondary,
	/// Filled using the tertiary color (`btn-tertiary`).
	Tertiary,
	/// Destructive action using the error color (`btn-error`).
	Error,
}

impl ButtonVariant {
	/// The semantic class name for this variant.
	pub fn class(&self) -> ClassName {
		match self {
			ButtonVariant::Filled => classes::BTN_FILLED,
			ButtonVariant::Outlined => classes::BTN_OUTLINED,
			ButtonVariant::Text => classes::BTN_TEXT,
			ButtonVariant::Tonal => classes::BTN_TONAL,
			ButtonVariant::Elevated => classes::BTN_ELEVATED,
			ButtonVariant::Secondary => classes::BTN_SECONDARY,
			ButtonVariant::Tertiary => classes::BTN_TERTIARY,
			ButtonVariant::Error => classes::BTN_ERROR,
		}
	}
}

/// A styled `<button>`; its content is the default slot's children.
#[template]
pub fn Button(variant: ButtonVariant) -> impl Bundle {
	rsx! {
		<button {Classes::new([classes::BTN, variant.class()])}><Slot/></button>
	}
}

/// A `<button>` sized for a single glyph (`btn-icon`); the slot is the icon.
#[template]
pub fn IconButton(variant: ButtonVariant) -> impl Bundle {
	rsx! {
		<button {Classes::new([classes::BTN, classes::BTN_ICON, variant.class()])}>
			<Slot/>
		</button>
	}
}

/// An `<a>` hyperlink styled as a button; its content is the default slot.
#[template]
pub fn Link(#[prop(into)] href: String, variant: ButtonVariant) -> impl Bundle {
	rsx! {
		<a {Classes::new([classes::BTN, variant.class()])} href={href}>
			<Slot/>
		</a>
	}
}

#[cfg(all(test, feature = "terminal"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::input::ButtonState;
	use bevy::input::keyboard::Key;
	use bevy::input::keyboard::KeyboardInput;

	/// Counts how many times the button's action fired (its `PointerUp` observer).
	#[derive(Resource, Default)]
	struct Activations(u32);

	/// The live stack a button needs: charcell render, focus, the document chain.
	fn button_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			bevy::input::InputPlugin,
			CharcellPlugin,
			RealtimeParsePlugin,
			DocumentPlugin,
			FocusPlugin,
		));
		app.init_resource::<Activations>();
		app
	}

	/// Spawn a `<Button>` whose action (a `PointerUp` observer) counts firings,
	/// returning the `<button>` entity. Activation (click or Enter) fires
	/// `PointerUp`, the same path `bx:click` rides.
	fn spawn_button(app: &mut App) -> Entity {
		app.world_mut()
			.spawn_template(rsx! { <div><Button>"Save"</Button></div> });
		app.update();
		let button = app
			.world_mut()
			.query::<(Entity, &Element)>()
			.iter(app.world())
			.find(|(_, element)| element.tag() == "button")
			.map(|(entity, _)| entity)
			.unwrap();
		app.world_mut().entity_mut(button).observe(
			|_: On<PointerUp>, mut count: ResMut<Activations>| count.0 += 1,
		);
		button
	}

	fn activations(app: &App) -> u32 { app.world().resource::<Activations>().0 }

	/// A `Button` widget is a real `<button>`, focusable, that fires its action on
	/// a pointer click.
	#[beet_core::test]
	fn button_is_focusable_and_fires_on_click() {
		let mut app = button_app();
		let button = spawn_button(&mut app);
		// the button is focusable (inferred from the tag)
		app.world().entity(button).contains::<Focusable>().xpect_true();
		// click: PointerUp fires the action
		let pointer = app.world_mut().spawn_empty().id();
		app.world_mut()
			.entity_mut(button)
			.trigger(PointerUp::new(pointer));
		app.update();
		activations(&app).xpect_eq(1);
	}

	/// Pressing Enter on a focused `Button` fires the same action (keyboard
	/// activation reuses the click path).
	#[beet_core::test]
	fn button_fires_on_enter() {
		let mut app = button_app();
		let button = spawn_button(&mut app);
		app.world_mut().entity_mut(button).insert(Focus);
		app.world_mut().write_message(KeyboardInput {
			key_code: bevy::input::keyboard::KeyCode::Enter,
			logical_key: Key::Enter,
			state: ButtonState::Pressed,
			text: None,
			repeat: false,
			window: Entity::PLACEHOLDER,
		});
		app.update();
		// the activate-on-enter system fired PointerUp on the focused button
		(activations(&app) >= 1).xpect_true();
	}
}
