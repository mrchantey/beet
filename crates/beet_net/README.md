# `beet_net`


This is a very tiny and simple replication library that can run on microcontrollers. It is extremely simple

## Features

### Incoming / Outgoing

Components, Events and Resources can be specified as incoming or outgoing.

### Multiple transports 
For instance a web bevy app can send `serde_json` messages to the dom and `bincode` messages to the server

## Limitations

- Components must be registered in the same order for every client
- Partial changes: on component or resource changes, the entire type is sent and applied
- Messages are not cached, if a client joins late it misses previous messages
- No authority determination
- Unidirectional Resources/Events: resources and events cannot be registered as both incoming and outgoing

## References

- [bevy_replicon](https://docs.rs/bevy_replicon/latest/bevy_replicon/)