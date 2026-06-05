+++
title = "beet_infra"
+++

# beet_infra

`beet_infra` extends the "everything is an entity" idea all the way out to the cloud. Infrastructure is declared as Bevy entities and exported to Terraform/OpenTofu JSON, so a beet app can plan, deploy and destroy the resources it runs on, all from within the same world that describes the app itself.

Building on [OpenTofu](https://opentofu.org) means beet does not reinvent the deploy engine; it provides the modelling layer above it. The crate is organised around a few concerns:

- `terra` builds and exports the OpenTofu JSON configuration.
- `bindings` ships pre-generated typed bindings for common providers, with a `bindings_generator` to derive new ones from a provider schema.
- the `deploy` feature adds actions for the full lifecycle: validate, plan, deploy, watch, show and destroy.

The deploy examples (`hello_lambda`, `hello_fargate`, `hello_lightsail`) take the router example and stand it up on AWS, which is the clearest demonstration of the payoff: the application and the infrastructure it lands on are described in the same language.
