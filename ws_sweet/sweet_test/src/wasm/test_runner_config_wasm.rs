use crate::test_runner::*;
use anyhow::Result;

impl TestRunnerConfig {
	pub fn from_deno_args() -> Result<Self> {
		let window = web_sys::window().expect("no global window exists");
		let deno = js_sys::Reflect::get(&window, &"Deno".into()).unwrap();
		let args = js_sys::Reflect::get(&deno, &"args".into()).unwrap();
		let args = js_sys::Array::from(&args)
			.iter()
			.map(|arg| arg.as_string().unwrap())
			.collect::<Vec<String>>();


		Ok(Self::from_raw_args(args.into_iter()))
	}

	// pub fn from_search_params() -> Self {
	// 	const FILTERS_KEY: &str = "f";
	// 	let quiet = SearchParams::get_flag("quiet");

	// 	let filters = SearchParams::get_all(FILTERS_KEY)
	// 		.iter()
	// 		.map(|f| Pattern::new(&format!("*{f}*")).unwrap())
	// 		.collect::<Vec<_>>();
	// 	let mut config = Self::default();
	// 	config.filters = filters;
	// 	config.quiet = quiet;
	// 	config
	// }
}
