+++
title = "Self-hosted mail"
+++

# Self-hosted mail

This is a runbook wearing a tutorial's clothes. The other tutorials teach beet's concepts from an empty crate and type out every line; this one stands up a real mail server on real infrastructure with a real domain, and it assumes you can fill in blanks, hold context across a week and read an error message without being told what it means.

It is also the longest-running thing in the docs. You start it on a Monday and finish it the following week, and most of that time is spent waiting on an Amazon support case. Plan for that rather than being surprised by it.

The shape is: [Stalwart](https://stalw.art) owns your mailboxes, your receiving and your policy, on one small box you control. Every outbound message relays through Amazon SES, so you never fight for the reputation of a single IP address. Postgres and S3 hold the state, the box itself is disposable, and the whole thing is declared as [`beet_infra`](/docs/crates/beet_infra) blocks in one `.bsx` file.

## 1. What you get, what it costs, what it demands

You get mailboxes whose message bodies sit in a bucket you own and whose metadata sits in a database you own. You get [JMAP](https://jmap.io) on port 443, which is the reason to do this at all if agents are going to read your mail: it is JSON over HTTP with batching and push, not a 1986 line protocol with a decade of extensions bolted on. You get IMAP, submission and CardDAV/CalDAV alongside it, so ordinary mail clients work with no special handling. And you get SES in front of delivery, which means your messages leave from IP addresses Amazon spends a great deal of money keeping trusted.

It costs about US$43 a month in Sydney on-demand pricing: EC2 `t4g.small` around $15, an elastic IP around $4, EBS around $3, an RDS `db.t4g.micro` around $16 plus $3 of storage, a couple of dollars of S3, and cents of SES. About $25 of that is mail-specific once you treat the database as company infrastructure that mail is merely the first tenant of. A savings plan takes roughly 30% off the compute later.

It demands four things:

- A domain, and a DNS provider with an API. This walkthrough uses Cloudflare, because the blocks speak its API and because being the registrar too makes DNSSEC one call instead of a registrar ticket.
- An AWS account, with an IAM user or role that can create EC2, RDS, S3, SES, SSM, IAM and CloudWatch resources.
- A deploy machine carrying `tofu`, `aws`, `ssh`/`scp`, `curl`, `openssl` and `wrangler`. Two of those are load-bearing in ways worth knowing up front: `openssl` mints the DKIM key you own, piping the public half out so the private half never touches your disk, and `curl` is the SMTP client, because beet has no native one yet.
- Patience for a support case argued in prose. See section 4.

What you do not get is escape from operations. This is a server that other servers connect to on port 25 at three in the morning, and it stays that way.

## 2. Build on a staging domain

This is the most useful idea in the tutorial, so it comes before any of the machinery.

You already have mail somewhere. That mail is how your invoices arrive and how your clients reach you, and it cannot wait two weeks while you learn what a Stalwart 0.16 config object looks like. So do not build on your apex domain. Build the entire system on `stalwart.<your domain>`, prove it, live in it, and move the apex across in one short window at the end.

The reason this works rather than merely deferring risk is that MX, SPF, DKIM and DMARC are all per-hostname records. A subdomain is a wholly separate mail domain as far as SMTP is concerned. Your `v=spf1 include:amazonses.com -all` at `stalwart.example.com` does not collide with your incumbent provider's SPF at the apex, because the two-record permerror only applies to two SPF records at the *same* name. A subdomain's own `_dmarc` takes precedence over the organizational domain's policy, so you can publish `p=reject` on the staging domain from the first deploy without imposing anything on the apex. Nothing you do during the build touches production mail.

One detail makes the eventual cutover short rather than a second build: the *box* is not a mail domain. Name it `mail.<your domain>` from the beginning and keep that name forever. Its reverse DNS record, its TLS certificate and its SMTP banner then never change across the cutover, which removes the riskiest churn from the riskiest step. Only the mail domains move.

Treat "no apex mail record before the cutover" as an invariant, not a preference. Write it as a test: every record name a staging block emits should be asserted to be *under* the apex and never at it. There is exactly one apex-scoped name a mail stack has any business publishing early, and it is not a mail record ([atproto](https://atproto.com) handle TXTs, if you use them).

## 3. Credentials and preflight

**Hand step.** Fill a `.env`:

```sh
AWS_ACCESS_KEY_ID=...
AWS_SECRET_ACCESS_KEY=...
AWS_REGION=ap-southeast-2
CLOUDFLARE_API_TOKEN=...
CLOUDFLARE_ZONE_ID=...
CLOUDFLARE_ACCOUNT_ID=...
TF_STATE_PASSPHRASE=$(openssl rand -base64 32)
```

The account variable beet reads is `CLOUDFLARE_ACCOUNT_ID`, not `CLOUDFLARE_DEFAULT_ACCOUNT_ID`. The passphrase is the one unrecoverable value in the file: it encrypts the OpenTofu state client-side, and state carries the database master password and the SES SMTP credential, because `sensitive = true` on a tofu variable redacts a value from plan and apply output and *not* from state. Back it up somewhere durable before you run anything.

Cloudflare token scopes, all narrowed to the one zone: Zone > DNS > Edit, Zone > Zone > Read, Account > Workers Scripts > Edit.

**Hand step, and worth doing before creating anything.** Prove every permission you are about to need, cheaply, in the order that fails fastest:

```sh
curl -s https://api.cloudflare.com/client/v4/user/tokens/verify \
  -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN"
aws sts get-caller-identity
```

Then one cheap read per AWS service you will touch (`ec2`, `rds`, `sesv2`, `ssm`, `iam`, `s3`, `logs`) in your chosen region, and a throwaway TXT record create and delete against the zone to prove DNS write rather than assume it. A missing permission discovered here costs seconds; the same permission discovered twenty minutes into an RDS provision costs twenty minutes and a partly-created stack.

*Gap:* this should be a `Preflight` action. It is the same check every stack wants and nothing in beet performs it yet.

**Hand step.** Enumerate the whole zone before you touch it, rather than reasoning about what ought to be in it:

```sh
curl -s "https://api.cloudflare.com/client/v4/zones/$CLOUDFLARE_ZONE_ID/dns_records?per_page=200" \
  -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN"
```

The default page size is 100, so a zone that outgrew one page will quietly report only its first hundred records. Ours held two records that were in nobody's plan: the ACM DNS-validation CNAMEs for the website's certificate, whose names are computed by ACM and can therefore only ever be *allowed as a pattern*. You will want that inventory again in section 6, when you write the audit allowlist.

**Hand step.** Sort out CAA before anything requests a certificate. If your zone carries an `amazonaws.com`-only CAA set, it covers ACM and *blocks* everything else, including the Let's Encrypt issuance Stalwart's ACME will need and Cloudflare's Universal SSL. Publish `issue` and `issuewild` rows for `letsencrypt.org`, `pki.goog; cansignhttpexchanges=yes` and `ssl.com` alongside the amazonaws pair. CAA is a zone-wide singleton: exactly one stack should own those rows, and every other block reconciles with them rather than emitting its own.

**Hand step.** Confirm your registrar. If the domain is registered at Cloudflare, enabling DNSSEC publishes the DS record to the parent automatically. Anywhere else and that is a manual registrar step you should schedule now rather than discover later.

Also stand up a website at the domain before you go further. This is not decoration: a human at Amazon opens that URL while reading your support case. Do it before section 4, not after.

## 4. SES production access, which is the long pole

Start this on day one. Everything else in this tutorial can proceed inside the SES sandbox, including the full deploy and both halves of the delivery probe, so the case runs in parallel with the build. But it gates real mail to real people, and it is measured in days.

The reality is a two-step negotiation, not a form.

**Step one** is the request in the SES console. Fill it in honestly and expect it to be auto-denied within a day, with a reply asking for detail and pointing out that you need a verified identity first. That is the normal path, not a rejection.

**Step two** is a written reply through the Support Center. It goes by hand: `aws support` requires a Business or Enterprise support plan and returns `SubscriptionRequiredException` on Basic, so there is no CLI path at any support tier worth paying for to get one.

Before you write it, make the claims true. Reviewers check, and a present-tense claim that is aspirational is the thing that turns a three-day case into a three-week one. So section 5's work, at minimum, happens before you reply: verified domain identities, DKIM verified, custom MAIL FROM verified, configuration sets with suppression and event publishing live, a DMARC record actually published, and a test message whose headers you have read.

A good reply is organised and specific. Cover, roughly in this order:

- **Who you are**, in two sentences, with the website they are about to open.
- **Your domains**, and in particular the staging arrangement. Explain that your existing mail must keep running, that you are commissioning on a subdomain which is an independent mail domain for SMTP purposes with its own MX, SPF, DKIM and DMARC, and that the apex is added as an identity once it is proven. This reads as competence rather than evasion, and it pre-empts the obvious question about why the apex is not the identity in the request.
- **What you send**, split by stream, one verified domain per stream. Be precise where the form's categories are not: "person-to-person correspondence written by staff from authenticated mailboxes" and "an opt-in publication to double opt-in subscribers" are two different things, and neither is quite what "Transactional" means. Say so.
- **Volume and cadence**, with a real number for today and a real number for a year out.
- **How lists are maintained.** State plainly that you hold no purchased, rented or scraped lists, and describe where addresses actually come from. For a publication, describe the confirmation step and say what you record about each consent.
- **Bounces, complaints and unsubscribes**, which is the section they care most about. Name the mechanisms: a configuration set per domain publishing bounce, complaint, reject and delivery-delay events to a topic; the account-level suppression list enabled for bounces and complaints; `List-Unsubscribe` and `List-Unsubscribe-Post` (RFC 8058 one-click) plus a visible footer link on anything bulk; and what you watch and what would make you pause sending.
- **Authentication**, concretely: DKIM key length, the custom MAIL FROM subdomain that makes SPF align with the From domain, the DMARC policy you publish, and the fact that you have confirmed all three passing end to end at a receiver rather than merely configured them.
- **A sample of the actual mail.** One representative message per stream, redacted. This is the part that distinguishes a real correspondent from a bulk sender, and it is worth more than any of the preceding paragraphs.

Send it within days. Idle cases auto-resolve, and while a resolved case can be reopened, momentum matters.

Ours was granted three days after the initial request and two after the detailed reply, moving the account out of the sandbox with a quota of 50,000 messages a day at 14 per second, effective immediately in the region. That grant is per account and per region, not per identity, so a grant earned while sending from a staging subdomain carries over to your apex at cutover with nothing to re-request. It does not carry to another region, which is worth knowing if you ever move.

*Not automatable.* A support case argued in prose is a human writing to a human. Do not plan around a future action for this one.

## 5. Identities, DKIM, MAIL FROM, configuration sets, and the topic

Everything in this section is eventually declared by `MailDomainBlock` and applied by tofu. Do it by hand first anyway, for the two sending domains, because the support reply in section 4 needs it to already be true and because doing it once by hand is how you learn what the block is doing on your behalf.

Per sending domain:

```sh
aws sesv2 create-email-identity --email-identity "$DOMAIN" \
  --dkim-signing-attributes NextSigningKeyLength=RSA_2048_BIT

aws sesv2 put-email-identity-mail-from-attributes --email-identity "$DOMAIN" \
  --mail-from-domain "bounce.$DOMAIN" --behavior-on-mx-failure USE_DEFAULT_VALUE
```

The enum is `RSA_2048_BIT`. `RSA_2048` is rejected.

Then publish, all DNS-only and none of them proxied:

- Three `<token>._domainkey.<domain>` CNAMEs to `<token>.dkim.amazonses.com`. The tokens come back from the create call.
- `MX bounce.<domain> 10 feedback-smtp.<region>.amazonses.com`
- `TXT bounce.<domain> v=spf1 include:amazonses.com -all`. The console suggests `~all`; `-all` is correct once you know that only SES sends for that name, and you do know that, because you just created the name.
- `TXT _dmarc.<domain> v=DMARC1; p=reject; rua=mailto:dmarc@<domain>`. SES never manages this one.

Verification on a zone you control authoritatively took under thirty seconds for us, not the hours the AWS documentation warns about. Those warnings are written for people whose DNS changes propagate through a provider they do not control.

Then a configuration set per sending domain, with reputation metrics on, per-set suppression for `BOUNCE` and `COMPLAINT`, and an event destination matching `BOUNCE`, `COMPLAINT`, `REJECT` and `DELIVERY_DELAY`. Make it the identity's default with `put-email-identity-configuration-set-attributes`. Reputation metrics being enabled is load-bearing rather than decorative: without it the CloudWatch metric never appears, and an alarm on a metric that never appears sits in `INSUFFICIENT_DATA` looking exactly like a healthy one.

All the sets can publish to one account-wide SNS topic. Events carry their configuration set, so splitting the topic only pushes routing from the consumer into IAM.

**The trap in this section**, and it is a good one, is the topic's access policy. The default policy on a new SNS topic is owner-only, and SES publishing into it fails *silently*: no error appears in SES, nothing appears in CloudWatch, and you discover it whenever you first go looking for a bounce that should have been recorded. The policy must grant `SNS:Publish` to the `ses.amazonaws.com` service principal under an `AWS:SourceAccount` condition.

**Prove the chain before you claim it.** In the sandbox you can only send to verified recipients, so verify a personal address as an `EMAIL_ADDRESS` identity, send one message from each domain with `aws sesv2 send-email`, and then read the headers. In Gmail that is "Show original", and what you want to see is SPF `PASS`, DKIM `PASS` with `d=` equal to the sending domain, and DMARC `PASS`. A partial pass is the interesting failure: DKIM alone still delivers to plenty of receivers, which is exactly how a broken SPF record ships unnoticed for months.

*Gap:* every step above except the support case is a `MailDomainBlock` field today, so on a second domain you would declare it rather than type it. The by-hand pass exists because of the ordering: the support reply needs verified identities, and the block cannot exist before you have written it. If you are doing this a second time, declare it.

**Hand step, with a judgement call attached.** You may publish the apex DKIM CNAMEs early if you want to. They are namespaced by selector token, so they cannot collide with your incumbent provider's selectors, and publishing early takes identity verification out of the cutover window entirely. We declined, on the grounds that "no apex mail record before cutover" is a rule worth keeping absolute, and because verification turned out to take seconds anyway. Either answer is defensible. Decide deliberately.

## 6. The stack itself

Now the automated part. One `.bsx` file declares the whole system, and the boundary between this section and the previous two is the honest answer to how much of this is automated.

```jsx
<Stack app_name="acme" region="ap-southeast-2">
	<DeployVerbs/>

	<VpcBlock label="net"/>
	<RdsPostgresBlock label="db" vpc="net" database="mail" consumers={["mail"]}/>
	<S3BucketBlock label="mail-blobs" object_versioning=true force_destroy=false runtime_write=true/>
	<S3BucketBlock label="mail-backups" object_versioning=true force_destroy=false
		runtime_write=true expire_days=180/>

	<StalwartBlock label="mail" hostname="mail.example.com"
		vpc="net" database="db" db_name="mail" db_user="postgres"
		blob_bucket="mail-blobs" backup_bucket="mail-backups" dns_stage="prod"
		ssh_public_key="ssh-ed25519 AAAA... deploy"/>

	<MailDomainBlock
		domain="stalwart.example.com"
		mail_host="mail.example.com"
		events_topic="acme-ses-events"
		dns_stage="prod"
		mailboxes={[
			{localpart:"pete", admin:true},
			{localpart:"probe"},
		]}
		aliases={[
			{localpart:"postmaster", target:"pete"},
			{localpart:"abuse", target:"pete"},
			{localpart:"dmarc", target:"pete"},
			{localpart:"tlsrpt", target:"pete"},
		]}/>
</Stack>
```

A domain is one `MailDomainBlock`, and everything composes from it: the SES identity, every record that makes the domain deliverable and discoverable, and the mailboxes it holds. A second domain is a second tag rather than an edit, which is exactly what makes the cutover in section 8 one more tag beside the others.

Some notes on the declarations that are not obvious from reading them:

**No NAT gateway.** The box lives in a public subnet with an elastic IP, and the database has nothing to call out to, so the private subnets are left on the VPC's main route table whose only route is the local CIDR. That is about $32 a month not spent, expressed as an absence rather than a setting. A private workload that genuinely needs one AWS service wants a VPC endpoint, not a gateway to the whole internet.

**The database security group emits ingress and no egress at all.** A security group declared with rules of its own loses the default allow-all egress rule, and for a database that is precisely right. It looks like an omission and is not.

**The box is cattle.** All state is in RDS and S3, machine config is cloud-init user data, and any change to that user data replaces the instance. Inbound SMTP during a rebuild is covered by sender retries, which run for days. Config that should *not* rebuild the box rides a fetch-at-boot pattern instead: the machine config holds SSM parameter *names* only, and an `ExecStartPre` script renders the real config from parameter store at every service start. Rotation is then `systemctl restart`, not a redeploy.

**The one static credential in the stack is the SES SMTP key pair**, because the SMTP protocol forces it. Terraform derives it into two SecureString parameters and it belongs to a dedicated IAM user whose only permission is `ses:SendRawEmail`. Everything else, including the instance's access to its own buckets, is an instance profile with IMDSv2 required.

**The ports are 22, 25, 443, 465, 587 and 993, and nothing else.** The management HTTP port is deliberately *not* among them: provisioning reaches it through an SSH tunnel, and the listener that serves it is destroyed at the end of the first provision.

**`dns_stage` is not decoration.** A mail name is not stack-composed: `mail.example.com` is the real name of real mail, so a second stage deploying the same declaration publishes a *second* record at that name and receivers round-robin between your live box and your experiment, taking production mail down without erroring anywhere. `dns_stage` names the one stage that owns the shared names; outside it the blocks emit their infrastructure and touch no record.

The deploy route is a sequence, and its order is the design:

```jsx
<Route path="deploy" {ExchangeSequence}>
	<EnsureSecret secret="db-password" variable="db_password"/>
	<EnsureSecret secret="mail-admin-password"/>
	<EnsureDkimKey/>
	<TofuApply/>
	<EipReverseDns/>
	<StalwartProvision ssh_key="~/.ssh/id_ed25519_mail"/>
	<MtaStsPublish/>
	<MailProbe mailbox="probe" sender_domain="news.example.com"/>
	<ZoneAudit/>
</Route>
```

Secrets are minted *before* the apply and handed to it as variables, so the database is created with its master password and the box boots reading its admin credential out of the parameter the same step wrote. `EnsureDkimKey` is create-if-missing for a sharper reason than the passwords: a rotated key under an already-published selector is a fortnight of unverifiable mail. The key is minted before the apply and the apply publishes its public half, so the selector the world resolves and the key the server signs with are one parameter read twice. Letting the server generate its own key would mean reading it back and publishing in a second apply, with a window in between where mail is signed by a selector nothing answers for.

Reverse DNS comes after the apply because AWS validates the forward record before publishing the reverse one. Provisioning comes after that, because Stalwart 0.16 keeps listeners, routing, domains and accounts as objects *inside* its data store rather than in any file terraform writes. The MTA-STS policy body is published after the apply that published the record pointing at it. The probe proves a message goes out and a message comes back authenticated. And the audit runs last, because it is the only check that can see what the deploy did *not* do: a record left behind by a block that stopped declaring it.

Two things about running these verbs:

`plan` and `deploy` do not produce the same diff, and reading a plan without knowing that is alarming. `deploy` runs the secret and DKIM steps before the apply and `plan` does not, so a plan will show the database password going to null and the DKIM record going to `p=` empty. Neither is real. The empty `p=` is worth recognising because it is also a genuine failure mode: `p=` with nothing after it is the wire form of a *revoked* key, and a bare `apply` that reaches past the deploy route will publish exactly that.

Run `validate`, then `plan`, then the audit, before you deploy anything. `plan` against real providers costs nothing and is the last cheap check before a phase that costs real money and real DNS. Ours reported 78 resources to add, 0 to change, 0 to destroy, with the planned DNS records matching the specification table exactly and nothing at the apex. Running `audit` before the first deploy is likewise worth it: against a zone of 30 records and a stack declaring 31 it reported clean, which means the allowlist was proven right while it was still free to be wrong.

The allowlist belongs on the stack rather than on the audit verb, so that the audit at the tail of `deploy` and the standalone `audit` route read the same list. Every entry is a decision with a reason attached, and on a staging build most of them retire at the cutover:

```jsx
<ZoneAudit allowed={[
	{name:"example.com", record_type:"MX",
		reason:"the incumbent provider serves the apex until cutover"},
	{name:"example.com", record_type:"TXT",
		reason:"the incumbent's apex SPF; a second SPF at this name would be a permerror"},
	{name:"_*.example.com", record_type:"CNAME",
		reason:"acm dns validation, name computed by acm"},
	{name:"mta-sts.stalwart.example.com",
		reason:"wrangler's worker custom domain: record and certificate both"},
]}/>
```

Two subtleties in writing one. Match DKIM selector names as *patterns*, because the SES selector token is computed and unknown until after the first apply, so the declared name is a terraform interpolation; collapsing `${...}` to a wildcard label keeps the audit useful *before* the first apply, which is exactly when a stray record does the most damage. And restrict a pattern by record type where you can, so that `_*.example.com` as a CNAME does not also swallow your `_dmarc`, `_mta-sts` and `_smtp._tls` TXTs.

## 7. The first deploy, the probe, and the soak

**Hand step.** Mint a dedicated deploy key before the first apply, because EC2 installs a key only at launch and swapping one later replaces the instance:

```sh
ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519_mail -N '' -C mail-deploy
```

The public half goes into `<StalwartBlock ssh_public_key=..>` and the path to the private half into `<StalwartProvision ssh_key=..>`. A dedicated key is the right answer here: reusing one that already authenticates somewhere else means one compromise reaches both.

**Hand step, if you did section 5 by hand.** Reconcile what you built by hand with what the stack now declares. SESv2 will not let terraform adopt an existing identity or configuration set silently, so the path is delete-and-let-the-apply-recreate: `aws sesv2 delete-email-identity`, `aws sesv2 delete-configuration-set`, and delete the hand-added zone records over the DNS API. Capture the configuration sets' event destinations first if you want to re-add them, and note that event destinations are create-only from terraform's point of view, so a hand-made one that exists in AWS but not in state fails the apply with `AlreadyExistsException`.

Then:

```sh
beet --main=infra/mail.bsx --stage=prod deploy
```

Budget an hour for the first one and expect it to fail at least once. The apply itself takes about twenty minutes, of which RDS is fifteen. A converged, idempotent re-deploy afterwards runs in under two minutes.

### When it fails

Some failure shapes worth recognising, because each cost us hours:

**The box is silent and provisioning times out.** `systemctl status stalwart` says the unit could not be found, which means cloud-init never got that far. `sudo cloud-init status` and then `/var/log/cloud-init-output.log` is the path. Ours was a tarball download dying mid-transfer on a transient network error. User data runs *once* per instance under `set -e`, so a flaky download is a permanently dead box that answers SSH perfectly: the service just does not exist. Fix it in the block (retry flags on the download) rather than on the box, and redeploy, which replaces the instance and proves the fix on a fresh boot.

**An instance replacement changes the host key**, and a stale `known_hosts` entry makes the next SSH fail with `REMOTE HOST IDENTIFICATION HAS CHANGED`. `ssh-keygen -R <ip>` between replacements.

**A config parse error reporting "line 1 column N".** Check whether N is the *length* of the file. An error at EOF means the parser consumed everything and a required field was missing at the top level, which is to say the document shape is wrong rather than a field in it being wrong.

**A partial apply is not a rolled-back apply.** One of ours replaced the instance and then errored on a later resource, leaving a rebuilt box with provisioning never run. Mail was down until the re-run, with inbound covered by sender retries. Killing a deploy mid-provision is safe by contrast: the apply has already committed its state and provisioning is idempotent, so the recovery is to fix the cause and run `deploy` again.

**A hung deploy is not necessarily a broken box.** SSH in and run `sudo ss -lntp`. A box listening on every declared port and none of the undeclared ones has recovered, and the problem is in what the deploy is waiting *for*. `systemctl is-active stalwart` and `test -f /etc/stalwart/config.json` are the other two lines worth running.

**ACME fails on a name that has no A record.** A mail *domain* is MX and TXT records with nothing resolving at the name itself, so it must not be a certificate SAN. Let's Encrypt fails that name and the entire order dies with it, including the valid names beside it. The SANs are exactly the names that resolve to the box: `autoconfig.<domain>`, `autodiscover.<domain>`, and the box hostname once. This failure is visible only in the server's own log, not in anything the deploy prints.

### The probe

`MailProbe` is the assertion that matters, and it works inside the SES sandbox. The outbound leg sends through the box's own submission listener with `curl` speaking implicit-TLS SMTP with AUTH, which exercises the listener exactly as a mail client would, TLS verification included. The inbound leg sends from your publication domain via `aws sesv2 send-email` to the probe mailbox, then reads the message back over JMAP and asserts its `Authentication-Results` header carries `spf=pass`, `dkim=pass` *and* `dmarc=pass`. Both simulator addresses and a once-verified recipient identity are permitted in the sandbox, so no production access is needed to prove the whole loop.

Assert all three verdicts, not any of them. A partial pass is how a broken SPF record ships unnoticed.

### Getting a password out

Every credential in this stack is generated and never displayed, which is right, and it leaves one real gap: setting up a mail client needs the value. That is what `export-passwords` is for.

```sh
beet --main=infra/mail.bsx --stage=prod export-passwords
```

It composes the parameter names off the declaration rather than making you type `/acme/prod/mail-account-<localpart>-at-<domain-with-dots-as-hyphens>` from memory, which is the part that goes wrong. It lists every mailbox on every served domain plus `admin@`, the account the server itself creates when the data store is first claimed and which no declaration names. `--infra` adds the database master password and the DKIM private keys, kept behind a flag because reading a mailbox password is setting up a client and reading the database password is an incident.

It prints secrets to stdout, deliberately and uniquely in this stack. Mind what is recording your session.

The hand version, for when you want one value and not all of them:

```sh
set -a && source .env && set +a
aws ssm get-parameter --region "$AWS_REGION" \
  --name "/acme/prod/mail-account-pete-at-stalwart-example-com" \
  --with-decryption --query Parameter.Value --output text
```

### Setting up a client

Thunderbird took the address and the password and discovered everything else from the autoconfig records, and ticking its address book and calendar sync worked against the same 443 with the same credentials, so CardDAV and CalDAV came along for free. The one prompt that needs explaining is the bare "mail.example.com requests username and password" from the DAV sync: it wants the *full address* as the username, with the same password.

### The soak, and a realistic expectation

Send a message to a personal Gmail address and read the headers. With SPF, DKIM (both signatures), DMARC under `p=reject` and a TLS 1.3 hop all passing, Gmail still filed our first message from the days-old domain into spam.

That is the correct outcome and it is worth stating plainly: authentication earns *deliverability*, and only history earns the *inbox*. There is no configuration that shortcuts it. The remedy is the soak itself, which is to mark it not-junk and then correspond normally for a couple of weeks. Plan for a partial repeat at cutover, since Gmail keys reputation off the From and DKIM domain, and that domain changes at the apex swap even though the SES addresses, the box and its reverse DNS all carry over whole.

Two more soak-period notes:

`watch`'s port-25 banner check cannot pass from a residential connection. ISPs block outbound 25 as an anti-spam measure, so the check times out looking exactly like a dead server while SES delivers through that same port perfectly well. Verify the banner from the box itself (`exec 3<>/dev/tcp/localhost/25 && head -1 <&3`), or trust the probe, and run `watch` from somewhere with port 25 egress if you run it at all.

`dig` is not on every machine. The dependency-free substitute, which is also the only check that means anything for DNS because it asks something other than your own provider's API:

```sh
curl -s -H 'accept: application/dns-json' \
  'https://cloudflare-dns.com/dns-query?name=<name>&type=TXT'
```

**Hand step.** Enable DNSSEC on the zone (`PATCH /zones/<id>/dnssec {"status":"active"}` on the Cloudflare API). If Cloudflare is also your registrar the DS record publishes to the parent automatically, but not quickly: hours after enabling, ours still had not appeared at the `.com` parent, and there is nothing to do but wait and re-query. Verify with a DoH query for `type=DS` at the apex, and an apex MX query checking the `AD` flag. The zone keeps resolving unsigned-style until the DS lands, so mail is unaffected either way.

**Hand step, and a judgement call.** MTA-STS ships in `testing` mode. Flip it to `enforce` after two clean weeks of TLS-RPT reports and not before. A `testing` policy withholds no mail whether or not it is fetched; an `enforce` policy withholds mail from senders whose TLS your box fails to satisfy, which is the point of it and also the risk. Also note that a wrangler custom domain's certificate is not instantly live at every edge: our deploy verified both policy fetches, a `curl` two minutes later got a TLS error from the same machine, and five minutes after that it was clean. Do not roll back over that.

## 8. Cutover

This section is a plan rather than a lived path: at the time of writing, our own cutover is behind a soak gate and has not run. Everything before this point has been executed and everything after it has. Treat the ordering rules as load-bearing and the timings as estimates.

The cutover changes *which* domain the working system serves and nothing else. The box, its IP, its reverse DNS, its certificate and its banner are all unchanged, which is why this is a short window rather than a second build.

Ahead of the window, each of these is safe on its own and none of them move mail:

1. Add the apex `MailDomainBlock` with `records={MailRecords::IdentityOnly}`, so the SES identity, DKIM and configuration set exist and verify while every record that would move mail stays unpublished.
2. Declare the apex addresses on your existing members and provision the mailboxes and aliases, empty. The server accepts nothing at them yet, because the apex MX still points at your incumbent.
3. Copy the mail across with an IMAP sync. Run it more than once; the last run happens inside the window.
4. Lower the apex TTLs a day ahead. On Cloudflare, records with the proxy off are TTL 1 meaning "auto", so confirm what the authoritative TTL actually is rather than assuming.
5. Write the rollback: the exact incumbent MX and SPF values, ready to paste back. Keep the incumbent account paid and live until the window is a week behind you.

The window itself, and the order matters:

6. **Replace** the apex SPF TXT with `v=spf1 include:amazonses.com -all`. There must be exactly one SPF TXT at the apex at every moment. This is a replace, never an add.
7. Publish `_dmarc`, `_mta-sts`, `_smtp._tls`, the autoconfig and autodiscover CNAMEs and the SRV records at the apex.
8. Final incremental IMAP sync.
9. Swap the apex MX to your box and delete the incumbent's MX records.
10. Run the probe against the apex. Smoke test both directions with a personal account. Watch the old mailbox for stragglers for a few days; sender retries cover the propagation gap in either direction.
11. Retire the incumbent's DKIM CNAMEs and drop them from the audit allowlist, so the audit goes back to asserting that the zone is exactly yours.
12. Keep the staging addresses as aliases for a deprecation window, then remove the staging block and let the audit clean up its records.

The rule behind the ordering is: sending first, receiving second. A half-applied cutover then degrades to "we can send but mail still arrives at the old provider", which is inconvenient, rather than to mail arriving nowhere.

## 9. Backups, and the drill

Start with the paragraph that justifies everything else in this section.

Our first restore drill found that the nightly `pg_dump` had never once produced a backup. Every check the stack makes was green: the timer was scheduled, the unit existed, the deploy passed, the zone audit was clean. The bucket had been empty since the day the timer was armed and nothing anywhere said so.

Nothing observes a backup except restoring one. Read that before you read any of the machinery below.

The layers, ordered by how much losing them would hurt:

1. **Postgres:** RDS automated backups with 14-day point-in-time recovery, roughly a five minute RPO, plus a final snapshot and deletion protection so that a stray `destroy` cannot eat the mail.
2. **Blobs:** S3 versioning, a lifecycle expiring noncurrent versions at 90 days, a public access block, and server-side encryption.
3. **Off-cloud:** a systemd timer on the box runs a nightly custom-format `pg_dump` into the backups bucket, with the bucket expiring objects at 180 days. The dump runs *on the box* rather than from a deploy machine, because a backup that only happens while somebody is deploying is not a backup, and it reads its credential from parameter store so that no secret rides its command line and a process listing on a mail box is not a credential dump.
4. **Everything else regenerates:** config from git, the box from user data, secrets re-mintable. The exception is the DKIM private key once its selector is published, so include it in the export.

One consistency rule to encode in your runbook: blobs are hash-keyed and the database references them, so a restored database must never be *newer* than the blob store. Back up the database first; restore the blobs first.

The off-cloud pull is `rclone sync :s3:<backups-bucket>/postgres <local>` against a read-only key. *Gap:* ours is documented and not scheduled, so until something runs it the sovereignty claim honestly stops at "in another AWS service".

### The two bugs, because both generalise

**A terraform reference resolved before the user data escape pass.** The whole user data string is escaped (`${` becomes `$${`) and then one deliberate reference is substituted in. A helper that resolves its *own* reference runs before that escape, so the escape turns a live reference into literal text, and the box ran `pg_dump --host '${aws_db_instance.x.address}'`. Every step that composes machine config must leave a *token* for the one late substitution, never a reference. Diagnosis is one line: `sudo cat` the script on the box and look at it.

**`PGSSLMODE=verify-full` needs `PGSSLROOTCERT=system` beside it.** We had installed the RDS CA bundle into `/etc/pki/ca-trust/source/anchors/` because Stalwart verifies through `rustls-platform-verifier`, which reads the OS trust store. libpq does not: it looks for `~/.postgresql/root.crt` and fails with "root certificate file does not exist" against a CA the machine trusts perfectly well. Two clients on one box, two different trust stores.

Also, `pg_dump` refuses a server newer than itself, so name the postgres client package from the database's own engine version. Get that wrong and the failure is a timer that fails quietly every night rather than a boot that fails loudly.

Useful commands: `systemctl list-timers stalwart-backup.timer` says when a timer will next run and whether it ever *has* (`LAST` is `-` if it never has), and `systemctl start <unit>` runs a timer's unit right now without disturbing the schedule. Verify by listing the bucket, not by the unit's exit code.

### The drill

The drill is a whole parallel stage, not a spare database. `deploy` it, `restore-drill` into it, `destroy` it. Ours ran seven minutes to deploy (57 resources), 28 seconds to restore a 176 KB dump, and a few minutes to tear down. Budget half an hour and one extra RDS instance.

Three things about building one over a shared zone, each of which is a consequence rather than a preference:

**The drill stage must not own any shared DNS name.** That is what `dns_stage` is for, and it is why the drill needs its own entry file with drill-scoped names, including a throwaway mail domain that exists only so that the box can obtain a certificate.

**The drill entry must not declare production's mail domains at all**, even guarded. An SES identity is account-global, so a second stack naming a domain the live stack owns fails its apply outright. Name the source domain as a *parameter* instead.

**What it must share is its labels and its app name.** A resource name composes as `<app>--<stage>--<label>`, so identical labels resolved against the source stage are how the drill finds production's backup bucket and the credential it signs in with.

Then the thing that reframes the whole exercise, which only appears once you actually run one. Stalwart 0.16 keeps its configuration *in the data store*, so a restore carries the source's entire identity: hostname, domains, listeners and certificates all live in the database the mail lives in. Ten seconds after `pg_restore` the drill box had stopped answering to its own name and was serving production's certificate for production's names.

A restore of this kind is not "the data came back". It is "the server came back", and restoring production's dump onto a second box produces a second production. Your runbook should say so.

Two consequences:

**The probe cannot be the drill's assertion.** A restored store carries the source's domains and accounts, so the drill box serves `probe@<source domain>` while that domain's MX still points at production: the probe's inbound leg would be answered by the live box and pass without the drill having restored anything. What *is* provable is that a restored account authenticates against the *drill* box and its mailbox is readable, which is what the drill action asserts itself.

**Reaching it means forcing the address.** No name both resolves at the drill box and is covered by the certificate it now holds, so `curl --resolve <autoconfig.source-domain>:443:<drill ip>` is how you dial it, with TLS verification *on*, against the restored certificate. Passing that is part of the proof: a box that had restored nothing could not present that certificate. Resolving the name normally would reach the live box and pass having tested nothing.

Smaller things the drill taught:

- `scp` cannot write into `/var/lib/stalwart`, which is `0700 stalwart:stalwart` and correctly so for a directory holding mail. The dump lands in the login user's home and is `install -o stalwart -g stalwart -m 0600`'d across. Generally: anything delivered into a hardened service directory is installed, not copied.
- Teardown leaves behind two things `destroy` cannot remove: the final DB snapshot (`skip_final_snapshot` is false, correctly) and the drill's SSM parameters, which actions create rather than terraform. Delete both, or the next drill collides on the snapshot name.
- While a drill stage is up, an audit of the production stage reports the drill's records as strays. That is by design and it resolves on teardown. Run the production audit after, not during.
- The drill does not prove the blob store. Message bodies are in S3 and only their metadata is in Postgres, so a drill with its own empty blob bucket proves the database came back and says nothing about the bodies. Versioning plus the public access block is the blob story, and it is a different rehearsal.

## What is automated, and what is not

The honest summary, because a tutorial that blurs this line strands its reader at the first step with no command.

**Fully declared and applied by beet:** the network, the database, both buckets, the box and its machine config, every SES identity and its DKIM and MAIL FROM records, the configuration sets with their suppression and their event destinations, the reputation alarms, every DNS record the mail domains need, the mail server's entire configuration (listeners, routing, domains, accounts, aliases, certificates), reverse DNS, the MTA-STS policy host and body, the delivery probe, the zone audit, the backup timer, the credential export and the restore drill.

**Genuinely not automatable, and labelled as such:**

- The SES production access case. A human argues it in prose to another human, and the Support API is behind a paid support plan anyway.
- A DS record at a registrar that is not your DNS provider.
- The judgement call about when to flip MTA-STS from `testing` to `enforce`.
- Deciding whether to publish apex DKIM early.

**Hand steps that are hand steps only because of ordering:** everything in section 5. The blocks declare all of it; you do it by hand the first time because the support reply needs it to be true before the stack exists.

**Gaps, which is to say the backlog:**

- A `Preflight` action doing the read-only permission sweep in section 3.
- An action to fetch a release asset, hash it and emit the version and digest constants, so that a version bump is one command.
- A native SMTP client, which would retire `curl` from the prerequisites and from the probe.
- Something that subscribes to the SES events topic. The events and the alarms both arrive there and, until you run `aws sns subscribe --protocol email`, nobody is paged.
- Adoption of the SNS topic itself, which is account-wide rather than stack-scoped and currently hand-made.
- A scheduled off-cloud `rclone` pull.
- Regenerating provider bindings is a manual command rather than a route.

## Where to go next

[`beet_infra`](/docs/crates/beet_infra) covers the blocks, the terraform export and the deploy lifecycle these verbs sit on. If you have not written a beet router before, [Speak every interface](/docs/tutorials/every-interface) is fifteen minutes and explains why a deploy verb and an HTTP route are the same thing.
