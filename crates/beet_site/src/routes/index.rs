use crate::prelude::*;
use beet::prelude::*;
use beet::rsx::sigfault::signal;
use serde::Deserialize;
use serde::Serialize;

pub fn get() -> RsxRoot {
	let val = 98;
	rsx! {
		<BeetPage>
			{val + 8}
			<span>hello world</span>
			<Counter client:load initial=7 />
			<style>
				span{
					color: red;
				}
			</style>
		</BeetPage>
	}
}


#[derive(Node, Serialize, Deserialize)]
pub struct Counter {
	initial: i32,
}

fn counter(props: Counter) -> RsxRoot {
	let (get, set) = signal(props.initial);


	rsx! {
		<div>
			{get.clone()}
			<button onclick=move |_|{set(get()+1)}>
			Increment
			</button>
		</div>
	}
}
