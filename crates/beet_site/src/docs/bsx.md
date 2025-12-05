## BSX - Bevy Scene XML

## Rationale

Beet started with `rsx`, a template tokenizer inspired by the web framework world where elements like `div` use lowercase 'intrinsic' identifiers and functional components like `Counter` use uppercase. For various reasons this is proving an antipattern in bevy development:
- Intrinsic elements makes documentation and type safety difficult, especially for external libraries
- Native contexts dont have intrinsic elements, even react native simply does not use lowercase elements
- The react compiler handles an element and a FC differently. In Beet this is not the case, `div` and `counter` are both regular bevy systems that output regular bevy scenes, there is no internal distinction between the two.
- Existing Bevy components and bundles are `TitleCase` rust structs and are also valid ways of defining an entity. The parser needs to know whether its dealing with a function or component because they are constructed differently.
- Beet is a bevy framework and the use of lowercase functions for authoring is the direction of the broader bevy community, conventions make everybodies life easier in the long run.

## Implementation

BSX does away with intrinsic elements completely, all lowercase elements are functions and the attributes are props. If one is missing, for example in an external html library, it must be explicitly defined. In rust this can easily be achieved with a short macro.

## Examples

1. Function without attributers
	```jsx
	// in
	<div/>
	// out
	div()
	```
2. Function with attributes
	```jsx
	// in
	<div hidden display='none'/>
	// out
	div(DivBuilder
		.hidden(true)
		.display("none")
		.build()
	)
	```
3. Struct without attributes
	```jsx
	// in
	<MyStruct/>
	// out
	MyStruct
	```
4. Struct with constructor
	```jsx
	// in
	<MyStruct::new(0.,1.) />
	// out
	MyStruct::new(0.,1.)
	```
5. Struct with default
	```jsx
	// in
	<MyStruct default/>
	// out
	MyStruct::default()
	```
6. Struct with attributes and default
	```jsx
	// in
	<MyStruct foo="bar" default/>
	// out
	MyStruct{
		foo: "bar",
		..default()
	}
	```