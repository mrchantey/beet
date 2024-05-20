# `beet_net`


This is a woefully naive but very tiny replication library that can run on microcontrollers.

## Limitations

- Components must be registered in the same order for every client
- Messages are not cached, if a client joins late it misses previous messages



## References

- [bevy_replicon](https://docs.rs/bevy_replicon/latest/bevy_replicon/)
