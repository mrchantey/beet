# beet_infra

Infrastructure as code for beet, built on [OpenTofu](https://opentofu.org).

Cloud resources are declared as Bevy entities and exported to Terraform/OpenTofu JSON, so a beet app can plan, deploy and destroy its own infrastructure. The deploy examples (`lambda`, `fargate`, `lightsail`) stand up the router example on AWS.

- `terra` - build and export OpenTofu JSON configurations
- `bindings` - pre-generated typed bindings for common providers
- `bindings_generator` - generate typed Rust bindings from a provider schema (`bindings_generator` feature)
- actions for the deploy lifecycle: validate, plan, deploy, watch, show, destroy (`deploy` feature)
