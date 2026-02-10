# Time for a rebuild


This repo is massive, its common to get weird mold compile errors. when they get too bad we do a full rebuild

## 1. Clear All

- run `just clear-all`
Absolutely clears all artefacts

## 2. Try full test suite

- run `just test-all`, use tail and 10m timeout to preserve context

The first few times this usually gets some compile errors like out of stack space, in those cases we can just run it again
if that still doesnt work a neat trick i sometimes use is bump the versions of all workspace crates in the root cargo.toml, ie bump all the `0.0.10-dev.3` just to unstuck the compiler
keep trying and debugging any compiler issues. keep in mind we are using `clang/mold` which is much faster than default but also more unstable, compiler issues like `increase stack size` are usually resolved by simply running again.
