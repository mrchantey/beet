+++
title= "Infrastructure"
+++

Beet comes with an [`sst`](https://sst.dev/) configuration for managing infrastructure.

## `beet infra`

This command is a lightweight wrapper of `npx sst`, with some added conventions. TLDR is the infrastructure step is seperaate

**infra directory**

Rust is currently not well supported in sst, for example it refuses to deploy without building a binary, but doesnt allow feature flags etc. The infra directory has some hacks to get around this, providing a dummy `Cargo.toml` and `bootstrap` file. For this reason `beet infra` will cd into the infra directory for all sst commands.


## Tips

This is the cleanest way I've found for managing infrastructure, even so there are some things to be aware of that I found not completely obvious:

### Dangling Resources



### State Mismatch

Running sst commands will keep a local record of the state of the application infrastructure. If these become out of sync, for example if a resource is manually created or removed, `sst deploy` will error. This can be fixed with `beet infra refresh`


## Custom Domains

sst can hook up custom domains, this guide will use cloudflare which has an excellent reputation as a domain registrar.

1. Purchase a domain at `https://domains.cloudflare.com/`
2. Configure the cloudflare environment variables:
	- `CLOUDFLARE_DEFAULT_ACCOUNT_ID`: https://dash.cloudflare.com/login > 'Bobs Account' hamburger > Copy Account ID
	- `CLOUDFLARE_API_TOKEN`: https://dash.cloudflare.com/profile/api-tokens > Create Token > edit zone DNS > specify zone (recommended)> Continue To Summary > Create Token > Copy
3. Update the `sst.config.ts`, see [this site's config](https://github.com/mrchantey/beet/blob/main/infra/sst.config.ts) for an example
4. Run `beet infra`