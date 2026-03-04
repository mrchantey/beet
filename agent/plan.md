# beet_node

Time to implement our markdown and html parsers.

- Refactor TextReader to have a 

```rust
pub struct TextReader{
	entity_stack: Vec<Entity>,
}

impl TextReader {
	pub fn new(entity: AsyncEntity){
		let root = entity.id();
		Self {
			entity_stack: vec![root],
			
		}
	}
}
```


These should both implement `TextParser`, ie 