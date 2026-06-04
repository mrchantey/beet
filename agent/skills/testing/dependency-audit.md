perform a dependency audit on each crates `Cargo.toml`. 
- ie `beet_net` should have very few required dependencies. http is one of many transports and should not be a hard requirement.
- remove unused/duplicate deps.
- dependencies that are shared amongst crates belong in the workspace Cargo.toml
- dependencies that are used by only one crate belong in that crates Cargo.toml
- if a dep would be a better fit in an upstream or downstream crate in the workspace, move it over.
- feature gate where possible, but avoid feature spaghettification

## Verification

- verify individual crates after making changes, including with/without the changed features.
- aggressively use tail for all tests to preserve context
