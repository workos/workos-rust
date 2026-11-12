# workos-workload-identity-gcp

GCP identity token provider for [WorkOS](https://workos.com) workload identity federation. Use it with the [`workos`](https://crates.io/crates/workos) crate to call the WorkOS API from GCP without a stored API key.

```bash
cargo add workos workos-workload-identity-gcp
```

```rust
use workos::{Client, WorkloadIdentityCredentials};
use workos_workload_identity_gcp::gcp_metadata;

let credentials = WorkloadIdentityCredentials::builder()
    .client_id("client_01...")
    .token_url("https://<your token endpoint>/oauth2/token")
    .environment_id("environment_01...")
    .identity_token_provider(gcp_metadata("https://api.workos.com"))
    .build()?;

let client = Client::builder().credentials(credentials).try_build()?;
```

The argument is the audience the identity token is requested with. It must match the audience of the trust rule configured in WorkOS.

The token comes from the GCP metadata server, so this works on Compute Engine, GKE, Cloud Run, Cloud Functions and App Engine.

See the [Workload Identity section of the `workos` README](https://github.com/workos/workos-rust#workload-identity) for the full guide.

## License

MIT.
