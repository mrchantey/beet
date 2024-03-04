# Topics

Topics are like urls in that they have parts. Each topic has a `scheme`, `path` and `key`.

- Scheme: defines how the topic is being used, ie `pubsub` or `request`. like HTTP, multiple schemes can be used for the same path.
- Path: is a heirarchical slash separated list of strings, ie `devices/my_device/sensors`
- Key: a u64 distinguishing between users of a topic, ie `0,1,2,3`.

```
request.my_device/sensors:3389
```

where 
  - `request` is the scheme
  - `my_device/sensors:3388` is the address
  - `my_device/sensors` is the path
  - `my_device` is a segment
  - `3388` is a key