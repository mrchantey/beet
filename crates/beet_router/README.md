# Beet Router


The Beet router is a file based router inspired by Astro, Next etc.

## Features
This crate runs in two modes, both disabled by default:


### Build
`--features=build`

This mode is used by `beet-cli` to scan a src directory, collect all file based routes and create a `file_router.rs` in the src src directory.

### Run

`--features=run`

The crate containing the `file_router.rs` should include this.