# workos-workload-identity-aws

AWS identity token provider for [WorkOS](https://workos.com) workload identity federation. Use it with the [`workos`](https://crates.io/crates/workos) crate to call the WorkOS API from AWS without a stored API key.

```bash
cargo add workos workos-workload-identity-aws
```

```rust
use workos::{Client, WorkloadIdentityCredentials};
use workos_workload_identity_aws::aws_sts;

let credentials = WorkloadIdentityCredentials::builder()
    .client_id("client_01...")
    .token_url("https://<your token endpoint>/oauth2/token")
    .environment_id("environment_01...")
    .identity_token_provider(aws_sts("https://api.workos.com", Some("us-east-1")).await?)
    .build()?;

let client = Client::builder().credentials(credentials).try_build()?;
```

The argument is the audience the identity token is requested with. It must match the audience of the trust rule configured in WorkOS.

The token comes from `sts:GetWebIdentityToken`, called with the AWS SDK's default credential chain on a regional STS endpoint. The AWS account needs outbound web identity federation enabled.

See the [Workload Identity section of the `workos` README](https://github.com/workos/workos-rust#workload-identity) for the full guide.

## License

MIT.
