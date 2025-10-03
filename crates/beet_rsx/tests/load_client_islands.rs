#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_dom::prelude::*;
use beet_rsx::prelude::*;
use beet_core::prelude::*;
use sweet::prelude::*;


#[test]
fn works() {
	let mut app = App::new();
	app.add_plugins(ApplyDirectivesPlugin)
		.register_type::<ClientIslandRoot<Counter>>();

	app.load_scene(SCENE).unwrap();
	app.query_once::<&NodeTag>().len().xpect_eq(2);
}


#[template]
#[derive(Reflect)]
fn Counter(initial: u32) -> impl Bundle {
	let (get, set) = signal(initial);
	rsx! {
		<p>"Count: "{get}</p>
		<button onclick=move || set(get() + 1)>"Increment"</button>
	}
}


const SCENE: &str = r#"
(
  resources: {},
  entities: {
    4294967299: (
      components: {
        "beet_dom::node::directives::client_island::ClientIslandRoot<load_client_islands::Counter, load_client_islands::Counter>": (
          value: (
            initial: 7,
          ),
        ),
        "beet_dom::node::directives::web_directives::ClientLoadDirective": (),
        "beet_dom::node::dom_idx::DomIdx": (0),
      },
    ),
  },
)"#;
