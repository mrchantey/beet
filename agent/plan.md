## beet_net -> add beet_tool dependency

See crates/beet_net/src/exchange/exchange.rs

This was created before `beet_tool`, the `exchange` pattern is architecturally a subset of a tool, so lets replace that pattern with the pattern of a tool where In=Request,Out=Response. 

This will involve the removal of lots of features from beet_net, just remove and do not mention them in the docs or mark deprecated. we are prerelease.

`beet_tool` also replaces the need for `beet_flow`, which is also architecturally a subset of beet_tool, see `beet_tool/src/control_flow`. We should also remove beet_flow integrations.



This change will break much of `beet_router`. thats ok, just leave beet_router and other downstream crates as broken.