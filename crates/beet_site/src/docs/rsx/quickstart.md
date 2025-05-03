---
title: Quickstart

sidebar:
  oder: 1
---

A counter is a great way to demonstrate core concepts, for a deeper dive read [how it works](./how-it-works)

<details>
<summary>Show final code</summary>
</details

Assuming rust nightly is installed, the following will get a new project up and running in four steps.


1. We'll start by creating a new project and adding sweet.
  ```sh
  cargo init hello-sweet
  cd hello-sweet
  cargo add sweet
  ```
2. Sweet uses file-based routes, lets create our index page.
  ```rust src/pages/index.rs
  use sweet::prelude::*;
  
  struct Index;
  
  impl Route for Index {
    fn rsx(self) -> Rsx {
      rsx!{
        <div>hello world!</div>
      }
    }
  }
  ```
3. Now lets run the server in the main function.
  ```rust src/main.rs
  use sweet::prelude::*;
  
  fn main(){
    SweetServer::default().run();
  }
  ```
4. Finally we'll run the sweet preprocessor and run our server.
  ```sh
  cargo binstall sweet-cli
  sweet parse 
  cargo run
  # server running at htp://127.0.0.1:3000
  ```

Visiting this page we get a heartwarming greeting, but now its time to make it iteractive. Lets create a counter component, a struct with the fields used as rsx props. A component is similar to a route but cannot use Axum extractors as props.

```rust src/components/Counter.rs
use sweet::prelude::*;
use sweet::prelude::set_target_text;

pub struct Counter{
  pub initial_value: usize
}
impl Component for Counter {
  fn rsx(self) -> impl Rsx {
    let mut count = 0;

    let onclick = |e| {
      count += 1;
      set_target_text(e, format!("You clicked {} times", count));
    }

    rsx!{
      <button onclick>You clicked 0 times</button>
    }
  }
}
```

To sweeten the developer flow this time we'll allow sweet to manage our run command with hot reloading.
```sh
sweet run
```
Nows a great time to check out the instant html reloading, lets move the text into a div.
```rust
rsx!{
  <div>You clicked 0 times</div>
  <button>Increment</button>
}
```

Sweet as! We have hot reloading but now our counter is broken. Lets fix this with a signal.

```rust
fn rsx(self) -> Rsx {
  let (count,set_count) = signal(0)
  
  let onclick = |e| {
    set_count.update(|c| c += 1 )
  }
  
  rsx!{
    <div>You clicked {count} times</div>
    <button onclick>Increment</button>
  }
}
```

<!-- ## Next steps

- If you want to ensure your counter doesn't go haywire check out this [testing guide]. 
- Beautify your counter with scoped styles or the built-in component library -->