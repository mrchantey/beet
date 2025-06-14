use beet_common::as_beet::*;

fn main() {
	let foo = FooSend::new(Foo { _inner: true });
	let _bar: FooSend<bool> = foo.clone();
}
#[derive(Sendit, Clone)]
#[sendit(derive(Clone))]
pub struct Foo<T: ToString>
where
	T: std::fmt::Display,
{
	_inner: T,
}
