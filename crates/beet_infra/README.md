# beet_infra

Infrastructure as code for beet, built on [OpenTofu](https://opentofu.org).

Cloud resources are declared as Bevy entities and exported to Terraform/OpenTofu JSON, so a beet app can plan, deploy and destroy its own infrastructure. The deploy examples (`lambda`, `fargate`, `lightsail`) stand up the router example on AWS.

- `terra` - build and export OpenTofu JSON configurations
- `bindings` - pre-generated typed bindings for common providers
- `bindings_generator` - generate typed Rust bindings from a provider schema (`bindings_generator` feature)
- actions for the deploy lifecycle: validate, plan, deploy, watch, show, destroy (`deploy` feature)

## Deploy layers

A deploy publishes into its stores and then rolls the service that reads them, so a deploy route applies once per phase: `<TofuApply layer="storage"/>` creates the resources the fill steps publish into (buckets, tables, the image registry), the fill steps run (image push, content sync), then a bare `<TofuApply/>` converges the whole stack and rolls the service. Blocks declare their publish-into resources with `Config::add_layer_resource`, defaulting the assignment to the `storage` layer and exposing it as a field. Naming a layer no block declares is a loud error, never a silent no-op.

Two footguns the markup cannot yet make unrepresentable:

1. A route with a single bare `<TofuApply/>` is valid markup that reproduces the fill race: the service rolls onto an image tag the push has not created yet and a store the sync has not filled, and the Fargate deployment circuit breaker (always on) rolls the deploy back instead of retrying until the fills land. Always order the layered apply and the fill steps before the bare apply.
2. A block that forgets `add_layer_resource` on a resource a fill step writes into wedges the first deploy rather than racing it: the fill fails loudly (eg `NoSuchBucket`) and aborts the sequence before the full apply that would have created the resource ever runs, with error text pointing at tofu rather than at the missing layer declaration.
