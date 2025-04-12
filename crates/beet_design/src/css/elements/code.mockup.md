# Code Block Style Test

## A rust code block

```rust
// A simple Rust function
fn fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() {
    let result = fibonacci(10);
    println!("The 10th Fibonacci number is: {}", result);
}
```

## Inline `code` Elements
When working with Rust variables like `let x = 42;` or functions like `fibonacci()`, it's important to remember that `u32` and `i32` are different types.

## Keyboard kbd Elements
Press <kbd>Ctrl + Shift + P</kbd> to open the command palette.

## Sample Output <samp>samp</samp> Elements
Terminal output: <samp>cargo build --release</samp>

## Mixed Inline Elements

The `Option<T>` type in Rust can be `Some(T)` or `None`. When you run the program you might see <samp>Hello, world!</samp> as output.

## Complex Code Block

```rust

// A more complex Rust example
use std::collections::HashMap;

struct Cache {
    data: HashMap<String, String>,
}

impl Cache {
    fn new() -> Self {
        Cache {
            data: HashMap::new(),
        }
    }
    
    fn insert(&mut self, key: &str, value: &str) -> Option<String> {
        self.data.insert(key.to_string(), value.to_string())
    }
    
    fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }
}

fn main() {
    let mut cache = Cache::new();
    cache.insert("name", "Rust");
    
    if let Some(value) = cache.get("name") {
        println!("Found: {}", value);
    }
}

```