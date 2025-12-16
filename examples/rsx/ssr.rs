//! An example of basic server-side rendering (SSR) with beet.
//!
//! ```sh
//! cargo run --example ssr --features=server,css
//! ```
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			RouterPlugin::default(),
		))
		.init_resource::<AppState>()
		.insert_resource(RenderMode::Ssr)
		.add_systems(
			Startup,
			|mut commands: Commands| {
				commands.spawn((HttpRouter, InfallibleSequence, children![
					middleware(),
					home(),
					html_bundle_to_response(),
				]));
			},
		)
		.run();
}

fn middleware() -> impl Bundle {
	OnSpawn::observe(|mut ev: On<GetOutcome>, mut state: ResMut<AppState>| {
		state.num_requests += 1;
		ev.trigger_with_cx(Outcome::Pass);
	})
}

fn home() -> EndpointBuilder {
	EndpointBuilder::get().with_handler(
		|_: In<()>, time: Res<Time>, state: Res<AppState>| {
			let uptime = format!("{:.2}", time.elapsed_secs());
			let num_requests = state.num_requests;

			rsx! {
				<Style/>
				<main>
					<div>hello world!</div>
					<div>uptime: {uptime} seconds</div>
					<div>request count: {num_requests}</div>
					// <a href="/hello-layer">visit jimmy</a>
					// <a href="/foo">visit foo</a>
					{
						match num_requests % 7 {
							0 => rsx! {
								<div>
								Congratulations you are visitor number {num_requests}!
								</div>
							}.any_bundle(),
							_ => ().any_bundle(),
						}
					}
				</main>
			}
		},
	)
}

#[derive(Debug, Default, Resource)]
struct AppState {
	num_requests: u32,
}

#[template]
fn Style() -> impl Bundle {
	// css is much easier to write with the rsx_combinator! macro
	// as many common css tokens like `1em` or `a:visited` are not valid rust tokens
	rsx_combinator! {r"
<style scope:global>
	main,article {
		padding-top: 2em;
		display: flex;
		flex-direction: column;
		align-items: center;
		height: 100vh;
	}
	a {
		color: #90ee90;
		margin: 0.5em;
	}
	a:visited {
		color: #3399ff;
	}
	body {
		font-size: 1.4em;
		font-family: system-ui, sans-serif;
		background: black;
		color: white;
	}
</style>"}
}
