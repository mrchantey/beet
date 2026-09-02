+++
title = "beet_infra"
+++

# beet_infra

`beet_infra` extends the "everything is an entity" idea all the way out to the cloud. Infrastructure is declared as Bevy entities and exported to Terraform/OpenTofu JSON, so a beet app can plan, deploy and destroy the resources it runs on, all from within the same world that describes the app itself.

Building on [OpenTofu](https://opentofu.org) means beet does not reinvent the deploy engine; it provides the modelling layer above it. The crate is organised around a few concerns:

- `terra` builds and exports the OpenTofu JSON configuration.
- `bindings` ships pre-generated typed bindings for common providers, with a `bindings_generator` to derive new ones from a provider schema.
- the `deploy` feature adds actions for the full lifecycle: validate, plan, deploy, watch, show and destroy.

The deploy examples (`lambda`, `fargate`, `lightsail`) take the router example and stand it up on AWS, so the application and the infrastructure it lands on are described in the same language.

Beyond the generic blocks, the `mail` feature carries a complete self-hosted mail stack: `MailDomainBlock` declares a mail domain end to end (its SES sending identity and every record that makes it deliverable), `StalwartBlock` declares the box, and the deploy actions beside them mint keys, set reverse DNS, configure the running mail server over its own API, publish the MTA-STS policy, probe delivery in both directions and audit the DNS zone. [Self-hosted mail](/docs/mail) walks the whole path, including the parts that are a human in a console rather than a beet action.
