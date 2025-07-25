use beet_core::as_beet::*;

fn main() {
	let foo = FooSendit::new(Foo { _inner: true });
	let _bar: FooSendit<bool> = foo.clone();
}
#[derive(Sendit, Clone)]
#[sendit(derive(Clone))]
pub struct Foo<T: ToString>
where
	T: std::fmt::Display,
{
	_inner: T,
}
