## crates/beet_node/src/parse/html

Great second pass! lets keep iterating

## Types

Our comment and doctype types are not set up correctly throughout the beet_node crate. lets change Comment to hold a String, and also add a Doctype(String) (usually "html") as well. 
This means updating `node_walker.rs`, adding visit_comment and visit_doctype methods. note that components are not mutually exclusive, its possible for a single entity to have a Doctype, element, comment and value. thats fine, just visit them in that order.

## html/diff.rs

- diff should be synchronous, ive updated `html/mod.rs` and the signature for diff_children, deliberately breaking the current impl. complete the refactor to use the world directly and make it all synchronous.

## Renderer

Lets also implement `crates/beet_node/src/render/html.rs`. diff.rs should provide an indication of what types are to be expected in the tree. Expression values should be rendered as strings verbatim

Add more options as required to the HtmlRenderer type.
