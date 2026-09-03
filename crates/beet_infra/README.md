# beet_infra

Infrastructure as code for beet, built on [OpenTofu](https://opentofu.org).

Cloud resources are declared as Bevy entities and exported to Terraform/OpenTofu JSON, so a beet app can plan, deploy and destroy its own infrastructure. The deploy examples (`lambda`, `fargate`, `lightsail`) stand up the router example on AWS.

- `terra` - build and export OpenTofu JSON configurations
- `bindings` - pre-generated typed bindings for common providers
- `bindings_generator` - generate typed Rust bindings from a provider schema (`bindings_generator` feature)
- actions for the deploy lifecycle: validate, plan, deploy, watch, show, destroy (`deploy` feature)

## Bindings

The committed provider bindings (`bindings`) are generated, never hand-edited: regenerate with `just bindings`. A provider bump is a two-line deliberate act, edit its `schema_version` in `terra::Provider` and rerun, which is what makes generation reproducible (`Provider::version` floats and would not).

Every generated resource implements `terra::ToJson`, whose output is a `beet_core::Value` rather than a `serde_json::Value`, so a rendered body enters `terra::Config` in the type the config already holds and no conversion sits at that boundary.

## Deploy layers

A deploy publishes into its stores and then rolls the service that reads them, so a deploy route applies once per phase: `<TofuApply layer="storage"/>` creates the resources the fill steps publish into (buckets, tables, the image registry), the fill steps run (image push, content sync), then a bare `<TofuApply/>` converges the whole stack and rolls the service. Blocks declare their publish-into resources with `Config::add_layer_resource`, defaulting the assignment to the `storage` layer and exposing it as a field. Naming a layer no block declares is a loud error, never a silent no-op.

Two footguns the markup cannot yet make unrepresentable:

1. A route with a single bare `<TofuApply/>` is valid markup that reproduces the fill race: the service rolls onto an image tag the push has not created yet and a store the sync has not filled, and the Fargate deployment circuit breaker (always on) rolls the deploy back instead of retrying until the fills land. Always order the layered apply and the fill steps before the bare apply.
2. A block that forgets `add_layer_resource` on a resource a fill step writes into wedges the first deploy rather than racing it: the fill fails loudly (eg `NoSuchBucket`) and aborts the sequence before the full apply that would have created the resource ever runs, with error text pointing at tofu rather than at the missing layer declaration.

## Declarations

A cloud resource is declared ONCE, in markup, as its provider block (`<S3BucketBlock label="app"/>`, `<DynamoTableBlock bx:ref="analytics" label="analytics"/>`). Both meanings hang off that one entity: the deploy meaning (the block's `Block` impl and `DeployRender` systems, always compiled) and the runtime meaning (a live store, attached by an `InfraPlugin` observer). Never derive a resource name a second time.

A block is a declaration, not a sequence step, so nothing dispatches it during a deploy. Anything a block must *do* belongs to whichever step consumes its output, never to the block being run: an artifact is built by the apply that uploads it (`TofuApplyAction` -> `BuildArtifact::build`), because a build wired as its own step is a build some other entry forgets, and an artifact uploaded but never built is a stale binary shipping under a green deploy.

Blocks are immutable components (reinsertion is the only mutation path). `ErasedBlock` is the data projection of any block (its label and artifact label), derived by a generic `on_insert` hook so it can never go stale, removed with its block, and one per entity: a second block type on the same entity raises a clobber error rather than silently retagging it, mirroring `beet_action`'s one-action-per-entity rule.

## The render

The deploy render is the `DeployRender` schedule. `RenderScope` is a component the seam (`RenderScope::render(world, entity)`, or `render_all` for every declared stack in one run) inserts on the stack root and takes back out, which is the whole reset. Every block system reaches UP to the nearest scope (`AncestorQuery<&mut RenderScope>`) and writes to it directly: `Declare` states grants and tofu variables, `Render` emits resources; computes lower the pool, so `Declare` runs first. `AccessGrants` is a sorted set by construction, so contribution order can never matter and a reordering of declarations never diffs a rendered policy. Render errors collect on the scope and fail the call before any tofu invocation, so one render reports every misconfiguration.

A block's data shape is the `Block` trait (`label`/`grants`/`variables`/`artifact_label`, never `dyn`, no world access). A simple block adds `EmitBlock::emit` and rides the generic `declare::<T>`/`render::<T>` systems; a block with cross-entity inputs (a relation, an artifact, the pool) writes its own render system in the same reach-up shape.

## Identity

A resource belongs to the stack it is authored under, and to no other: `StackQuery::declared` is the stack root's inclusive descendants, full stop, with no adoption sweep. A declaration outside every `<Stack>` still resolves the process default for its runtime meaning, but no deploy provisions it. Load-bearing identity rides `<Stack>`, a component registered in every native build, never a template prop: a prop is absent exactly in the binary that did not link the template, which is a lean boot silently resolving the wrong stage.

A declaration carries only its **label**, and identity is two types: the authored `Stack` (every field optional, falling back to the nearest ancestor `<Stack>` else the process) and the total `ResolvedStack` it resolves to, the only type that composes `<app>--<stage>--<label>`. App identity lives in exactly one place, `PackageConfig::app_name`. Reach both through `StackQuery`; one-launch mechanics (the state backend, the work dir, the deploy id every artifact is keyed by) live on the `Deployment` resource. See the `Stack` docs for the whole model.

A store (`S3Store`, `DynamoStore`) sits below `Stack` in the crate graph, so it cannot resolve an ancestor stack. Stack-relative resolution belongs to the declaration, whose attach observer hands the resolved region in; the only public store constructor makes a caller name a region.

## References

A runtime consumer names the declaration through a relation, never a name: `<Router {(AnalyticsConfig, StoreRef($analytics))}>`. A consumer with nothing to point at is a loud error naming the relation it is missing, not a silent local fallback. One relation per store the consumer distinguishes, purpose-named (`StoreRef` is the primary; the rollup job adds `RollupStoreRef` and `ArchiveStoreRef`), since a relationship is one-per-type and a consumer touching three stores must say which is which. `StoreRef::resolve` reads the erased store off any of them, backing off first because a declaration's runtime half lands through the command queue.

A deploy-time reference from one block to another rides the same relation model (`VpcRef`, `DatabaseRef`, `InvokeTarget`, each targeting the declaration entity via `bx:ref`): the target block owns the ident composition, the consumer's render system reads the target's block through the relation and asks it to compose, and a missing, dangling or out-of-stack target is a collected render error naming the relation, before `tofu plan` ever runs. `SecretRef` stays a name composition (`/app/stage/label`): a secret is created by `EnsureSecret` at deploy time, so there is no declaration entity to target.

## Grants

Permissions are declared by the resource (`Block::grants` -> `AccessGrant`) and **lowered** by the compute block, which for the AWS computes is the shared `IamPolicy`: it seeds the statements the compute needs on its own account (an artifacts bucket, a log group) and lowers the stack's grants into read/write bucket and per-table statements. A compute whose lowering yields nothing emits no policy resource at all, never a managed `FullAccess` one; per-compute needs ride knobs on the shared core, never forks. A resource block never writes an ARN; a compute block never names a sibling resource.

An `AccessGrant` is `{kind, name, permissions}`: `kind` is a plain string constant the declaring block owns (`S3BucketBlock::ACCESS_KIND`), so a new provider mints `"r2_bucket"` without touching shared code, and the ARN region comes from the compute's own resolved stack. Lowering is **loud on unknown**: a kind the compute cannot lower fails the deploy naming both the kind and the compute, and there are no `_ =>` catch-alls in grant handling (a silently dropped grant is a box that serves until the first request touching that resource).

## Bucket profiles

A bucket is one of three profiles, and the label names the profile:

- `app` is deploy-mirrored content: the sync owns that tree (and prunes what it did not put there) and the runtime holds a read-only grant on it. The `assets` buckets (public, stage-shared) are a public variant of this profile.
- `archive` is its inverse (`<S3BucketBlock label="archive" runtime_write=true object_versioning=true deploy_versioned=false/>`): append-only cold data no deploy sync ever touches and the only bucket the running process may write. It holds backup dumps AND sole-copy archives (the analytics raws, whose table rows expire once archived), which is why it is not called `backups`: a bucket named for backups invites treating a sole copy as expendable.
- `user-data` is reserved for beet-owned runtime-written primary data of any origin (chat blobs, ingested mail bodies), declared when its first consumer lands, never unconsumed.

Sharing a bucket means sharing a **write grant**. A grant is whole-bucket (`Block::grants` names the bucket, never a prefix), so prefix ownership inside one is writer convention and nothing more: co-locate two datasets only when their writers already trust each other, and give data with its own trust boundary its own bucket and its own grants. What a prefix CAN say is retention: `expire_prefixes={[{prefix:"postgres/", expire_days:180}]}` renders one filtered lifecycle rule beside the unfiltered `expire_days`/`expire_noncurrent_days` ones, so an `archive` expires a nightly database dump while the sole copy beside it is kept forever. The trailing slash is load-bearing (`postgres` also matches `postgres-old/`), a rule id derives from its prefix rather than declaration order, and a zero-day or id-colliding rule fails at render rather than at apply.

Expiry is the **last step of a retention order**, never a policy on its own: `<DynamoTableBlock ttl="ttl"/>` enables only the attribute DynamoDB reads an expiry from, and a row without that attribute is never deleted, so the writer decides what is expirable. The reference order is the analytics rollup (`AnalyticsRollupRun` in `beet_net`): archive the day cold, aggregate it into rows that carry no `ttl`, confirm both by reading them back, and only then stamp that day's raws.

## Scheduled jobs

A recurring job is a declaration like any other: `<ScheduledJobBlock label="rollup-daily" {InvokeTarget($rollup_fn)} schedule="cron(0 3 * * ? *)" path="rollup"/>` renders an EventBridge schedule plus an invoke role scoped to the one function its `InvokeTarget` names, and declares no grants of its own (what the job may touch is the target lambda's own declared grants). The cron expression is validated at render, so a typo fails the deploy rather than the job. What an invoke delivers is a `ScheduledInvoke` (`beet_net`), the ONE payload type the block serializes and the lambda adapter deserializes, tagged and versioned because a schedule keeps invoking with the payload the last deploy rendered; its status resolves the INVOCATION rather than a response body, since a job whose dispatch failed must fail the invoke or it reports green in every metric.

A job lambda publishes nothing: `<LambdaJobBlock label="rollup" exec_route="jobs" features=".."/>` is the invoke-only counterpart of `<LambdaSiteBlock/>`, rendering the function, role, log group and lowered policy but NOT the function url, gateway, route, stage or anonymous invoke permission (a hostname on such a block fails the deploy). The runtime offers no argv, so `remote_bootstrap` bakes into the zip's `bootstrap` and `exec_route` names the verb whose `Router` the invoke's path dispatches into: the job runs a route of the same entry document the site serves from, in its own url space and its own process, so co-residence in one file costs the served site no reachable surface.
