Lets keep iterating on beet_net

- remove handler_exchange and handler_exchange_async, just use func_tool and async_tool directly. this should  also fix the http_server example which isnt compiling rn.

verify all examples are running in examples/net. use timeout for ones like server, they wont close on their own.
