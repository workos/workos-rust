# workos-workload-identity-github-actions

GitHub Actions identity token provider for [WorkOS](https://workos.com) workload identity federation. Use it with the [`workos`](https://crates.io/crates/workos) crate to call the WorkOS API from GitHub Actions without a stored API key.

```bash
cargo add workos workos-workload-identity-github-actions
```

```rust
use workos::{Client, WorkloadIdentityCredentials};
use workos_workload_identity_github_actions::github_actions;

let credentials = WorkloadIdentityCredentials::builder()
    .client_id("client_01...")
    .token_url("https://<your token endpoint>/oauth2/token")
    .environment_id("environment_01...")
    .identity_token_provider(github_actions("https://api.workos.com"))
    .build()?;

let client = Client::builder().credentials(credentials).try_build()?;
```

The argument is the audience the identity token is requested with. It must match the audience of the trust rule configured in WorkOS.

The workflow or job needs `permissions: id-token: write`. Choose the TLS backend with the `rustls-tls` (default) or `native-tls` feature, as in `workos`.

See the [Workload Identity section of the `workos` README](https://github.com/workos/workos-rust#workload-identity) for the full guide.

## License

MIT.
