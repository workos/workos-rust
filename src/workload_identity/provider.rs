// @oagen-ignore-file
//! [`IdentityTokenProvider`] and the providers that ship in the SDK:
//! [`token_file`], [`token_env`], and any closure.

use std::error::Error as StdError;
use std::fmt;
use std::future::Future;
use std::path::PathBuf;

use async_trait::async_trait;

/// The error an [`IdentityTokenProvider`] returns. Any error type converts
/// into it with `?` or `.into()`.
pub type ProviderError = Box<dyn StdError + Send + Sync + 'static>;

/// Returns the workload's identity token: the JWT its platform issued.
///
/// The SDK calls the provider once per exchange attempt and retries a failing
/// provider with backoff, so an implementation holds no cache and does no
/// retries of its own. It must never put the token in an error.
///
/// Implemented for any `Fn() -> impl Future<Output = Result<String, E>>`, so a
/// callback needs no struct:
///
/// ```
/// use workos::WorkloadIdentityCredentials;
///
/// # fn build() -> Result<(), workos::WorkloadIdentityError> {
/// let credentials = WorkloadIdentityCredentials::builder()
///     .client_id("client_123")
///     .token_url("https://auth.example.com/oauth2/token")
///     .environment_id("environment_123")
///     .identity_token_provider(|| async {
///         // Fetch the workload's JWT from wherever the platform keeps it.
///         Ok::<_, std::io::Error>(std::fs::read_to_string("/var/run/token")?)
///     })
///     .build()?;
/// # let _ = credentials;
/// # Ok(()) }
/// ```
#[async_trait]
pub trait IdentityTokenProvider: Send + Sync {
    /// Return the identity token.
    async fn token(&self) -> Result<String, ProviderError>;
}

#[async_trait]
impl<F, Fut, E> IdentityTokenProvider for F
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = Result<String, E>> + Send,
    E: Into<ProviderError>,
{
    async fn token(&self) -> Result<String, ProviderError> {
        (self)().await.map_err(Into::into)
    }
}

/// What went wrong in [`token_file`] or [`token_env`]. Names the file or the
/// variable, never its contents.
#[derive(Debug)]
struct SourceError(String);

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl StdError for SourceError {}

/// Reads the identity token from a file. See [`token_file`].
#[derive(Debug, Clone)]
pub struct TokenFile {
    path: PathBuf,
}

/// An [`IdentityTokenProvider`] that reads the token from `path`.
///
/// The file is read again on every exchange, so a platform that rotates the
/// file (a projected Kubernetes service account token, for example) is picked
/// up without a restart. Surrounding whitespace is trimmed. A missing or empty
/// file is an error.
pub fn token_file(path: impl Into<PathBuf>) -> TokenFile {
    TokenFile { path: path.into() }
}

#[async_trait]
impl IdentityTokenProvider for TokenFile {
    async fn token(&self) -> Result<String, ProviderError> {
        // A small local read once per exchange; not worth tokio's `fs` feature.
        let contents = std::fs::read_to_string(&self.path).map_err(|e| {
            SourceError(format!(
                "could not read the identity token file {}: {}",
                self.path.display(),
                e.kind()
            ))
        })?;
        let token = contents.trim();
        if token.is_empty() {
            return Err(SourceError(format!(
                "the identity token file {} is empty",
                self.path.display()
            ))
            .into());
        }
        Ok(token.to_string())
    }
}

/// Reads the identity token from an environment variable. See [`token_env`].
#[derive(Debug, Clone)]
pub struct TokenEnv {
    name: String,
}

/// An [`IdentityTokenProvider`] that reads the token from the environment
/// variable `name`.
///
/// The variable is read again on every exchange. An unset or empty variable
/// is an error.
pub fn token_env(name: impl Into<String>) -> TokenEnv {
    TokenEnv { name: name.into() }
}

#[async_trait]
impl IdentityTokenProvider for TokenEnv {
    async fn token(&self) -> Result<String, ProviderError> {
        match std::env::var(&self.name) {
            Ok(value) if !value.trim().is_empty() => Ok(value),
            _ => Err(SourceError(format!(
                "the environment variable {} is unset or empty",
                self.name
            ))
            .into()),
        }
    }
}
