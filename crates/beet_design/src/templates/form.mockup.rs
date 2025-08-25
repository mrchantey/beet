use crate::prelude::*;
use bevy::prelude::*;
use bevy::reflect::DynamicStruct;



pub fn get() -> impl Bundle {
	rsx! {
		<Inner client:load/>
	}
}


#[template]
#[derive(Reflect)]
pub fn Inner() -> impl Bundle {
	#[derive(Clone, Reflect)]
	struct MyStruct {
		name: String,
	}

	let (get, set) = signal(None);

	let onsubmit = move |data: DynamicStruct| {
		let val = MyStruct::from_reflect(&data).unwrap();
		set(Some(val));
	};

	rsx! {
		<article>
		<Form onsubmit_dyn=onsubmit>
			<TextField name="name"/>
			<Button type="submit">Submit</Button>
			<div>
			{move||{
				match get(){
				Some(val)=>{
					rsx!{hello {val.name}}.any_bundle()
				}
				None=>{
					rsx!{Please submit the form}.any_bundle()
				}
				}
			}}
			</div>
		</Form>
		</article>
	}
}
