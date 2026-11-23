//! AWS identity token provider for WorkOS workload identity federation.
//!
//! [`aws_sts`] returns a provider that asks AWS STS for a web identity token
//! (`sts:GetWebIdentityToken`) for the workload's IAM principal. Pass it to
//! `workos::WorkloadIdentityCredentials`:
//!
//! ```no_run
//! use workos::{Client, WorkloadIdentityCredentials};
//! use workos_workload_identity_aws::aws_sts;
//!
//! # async fn build() -> Result<(), Box<dyn std::error::Error>> {
//! let credentials = WorkloadIdentityCredentials::builder()
//!     .client_id("client_123")
//!     .token_url("https://auth.example.com/oauth2/token")
//!     .environment_id("environment_123")
//!     .identity_token_provider(aws_sts("https://api.workos.com", Some("us-east-1")).await?)
//!     .build()?;
//!
//! let client = Client::builder().credentials(credentials).try_build()?;
//! # let _ = client;
//! # Ok(()) }
//! ```
//!
//! The AWS account must have outbound web identity federation enabled, and
//! the principal needs permission for `sts:GetWebIdentityToken`.

use std::error::Error as StdError;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_sdk_sts::config::retry::RetryConfig;
use aws_sdk_sts::config::timeout::TimeoutConfig;
use aws_sdk_sts::error::{DisplayErrorContext, ProvideErrorMetadata, SdkError};
use workos::workload_identity::{IdentityTokenProvider, ProviderError};

/// The signing algorithm WorkOS accepts for AWS-issued tokens.
const SIGNING_ALGORITHM: &str = "RS256";
/// The pseudo-region that selects the global STS endpoint.
const GLOBAL_REGION: &str = "aws-global";
/// How long connecting to STS may take, and how long the one request may
/// take from start to finish.
const TIMEOUT: Duration = Duration::from_secs(10);

/// An identity token provider for workloads running on AWS.
///
/// `audience` is the `aud` claim the token is requested with; it must match
/// the audience of the trust rule in WorkOS. The conventional value is
/// `https://api.workos.com`.
///
/// `region` selects the regional STS endpoint. Pass `None` to use the region
/// the AWS SDK resolves from the environment, the shared config file or
/// instance metadata. It is an error if there is no region:
/// `GetWebIdentityToken` is not served by the global STS endpoint.
///
/// The STS client is built once, here, from the AWS SDK's default credential
/// chain, with the AWS SDK's own retries switched off. One call to the
/// provider is one request to STS, with a 10-second limit on connecting and
/// a 10-second limit on the whole request. The provider keeps no cache and
/// does not retry; the WorkOS SDK retries a failed call as part of the token
/// exchange.
pub async fn aws_sts(audience: impl Into<String>, region: Option<&str>) -> Result<AwsSts, Error> {
    let mut loader = aws_config::defaults(BehaviorVersion::latest());
    if let Some(region) = region.filter(|region| !region.is_empty()) {
        loader = loader.region(Region::new(region.to_owned()));
    }
    let config = loader.load().await;
    let sts = aws_sdk_sts::Client::from_conf(sts_config(&config)?);
    Ok(AwsSts {
        audience: audience.into(),
        sts: Arc::new(sts),
    })
}

/// The STS client configuration: the shared AWS configuration, once it is
/// known to name a region, with this client's own retry and timeout settings.
/// With a region the AWS SDK resolves the regional endpoint
/// (`sts.<region>.amazonaws.com`), never the global one.
fn sts_config(config: &SdkConfig) -> Result<aws_sdk_sts::Config, Error> {
    let region = config.region().map(Region::as_ref).unwrap_or_default();
    if region.is_empty() {
        return Err(Error::new(
            "no AWS region: pass one to aws_sts, or set AWS_REGION. sts:GetWebIdentityToken needs a regional STS endpoint",
        ));
    }
    if region == GLOBAL_REGION {
        return Err(Error::new(
            "the AWS region aws-global selects the global STS endpoint, which does not serve sts:GetWebIdentityToken; use a real region",
        ));
    }
    Ok(aws_sdk_sts::config::Builder::from(config)
        // Providers do not retry: the WorkOS SDK retries the provider call as
        // part of the token exchange, and the two would multiply.
        .retry_config(RetryConfig::disabled())
        // The AWS SDK sets no limit on a whole request. The token exchange
        // that calls the provider has a deadline, and a stalled STS must not
        // hold a caller past it.
        .timeout_config(
            TimeoutConfig::builder()
                .connect_timeout(TIMEOUT)
                .operation_timeout(TIMEOUT)
                .build(),
        )
        .build())
}

/// The one STS call the provider makes. Tests replace it with a stub.
#[async_trait]
trait Sts: Send + Sync {
    async fn web_identity_token(
        &self,
        audience: &str,
        signing_algorithm: &str,
    ) -> Result<Option<String>, StsFailure>;
}

/// An STS failure reduced to what is safe to print: the AWS error code and a
/// message. The raw response is dropped.
struct StsFailure {
    code: Option<String>,
    detail: String,
}

#[async_trait]
impl Sts for aws_sdk_sts::Client {
    async fn web_identity_token(
        &self,
        audience: &str,
        signing_algorithm: &str,
    ) -> Result<Option<String>, StsFailure> {
        // `DurationSeconds` is left unset, so STS applies its default.
        let result = self
            .get_web_identity_token()
            .audience(audience)
            .signing_algorithm(signing_algorithm)
            .send()
            .await;
        match result {
            Ok(output) => Ok(output.web_identity_token().map(str::to_owned)),
            Err(error) => Err(StsFailure {
                code: error.code().map(str::to_owned),
                detail: match &error {
                    SdkError::ServiceError(_) => error.message().unwrap_or_default().to_owned(),
                    // The raw response rides along with this variant; keep it out.
                    SdkError::ResponseError(_) => {
                        "the response from STS could not be read".to_owned()
                    }
                    other => DisplayErrorContext(other).to_string(),
                },
            }),
        }
    }
}

/// The provider [`aws_sts`] returns.
pub struct AwsSts {
    audience: String,
    sts: Arc<dyn Sts>,
}

impl AwsSts {
    async fn fetch(&self) -> Result<String, Error> {
        match self
            .sts
            .web_identity_token(&self.audience, SIGNING_ALGORITHM)
            .await
        {
            Ok(Some(token)) if !token.is_empty() => Ok(token),
            Ok(_) => Err(Error::new("sts:GetWebIdentityToken returned no token")),
            Err(failure) => Err(Error::new(format!(
                "sts:GetWebIdentityToken failed ({}): {}",
                failure.code.as_deref().unwrap_or("no AWS error code"),
                failure.detail
            ))),
        }
    }
}

#[async_trait]
impl IdentityTokenProvider for AwsSts {
    async fn token(&self) -> Result<String, ProviderError> {
        Ok(self.fetch().await?)
    }
}

impl fmt::Debug for AwsSts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AwsSts")
            .field("audience", &self.audience)
            .finish_non_exhaustive()
    }
}

/// Why the AWS provider could not be built or could not return a token. The
/// message never contains a token.
#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl StdError for Error {}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use aws_sdk_sts::config::endpoint::{DefaultResolver, Params, ResolveEndpoint};
    use aws_sdk_sts::config::{Credentials, SharedCredentialsProvider};
    use wiremock::matchers::any;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    const AUDIENCE: &str = "https://api.workos.com";

    /// Stands in for STS: records what it was asked and answers from a script.
    struct StubSts {
        calls: Mutex<Vec<(String, String)>>,
        answer: fn() -> Result<Option<String>, StsFailure>,
    }

    impl StubSts {
        fn new(answer: fn() -> Result<Option<String>, StsFailure>) -> Arc<Self> {
            Arc::new(Self {
                calls: Mutex::new(Vec::new()),
                answer,
            })
        }

        fn calls(&self) -> Vec<(String, String)> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl Sts for StubSts {
        async fn web_identity_token(
            &self,
            audience: &str,
            signing_algorithm: &str,
        ) -> Result<Option<String>, StsFailure> {
            self.calls
                .lock()
                .unwrap()
                .push((audience.to_owned(), signing_algorithm.to_owned()));
            (self.answer)()
        }
    }

    fn provider(stub: &Arc<StubSts>) -> AwsSts {
        AwsSts {
            audience: AUDIENCE.to_owned(),
            sts: stub.clone(),
        }
    }

    fn sdk_config(region: Option<&'static str>) -> SdkConfig {
        let mut builder = SdkConfig::builder().behavior_version(BehaviorVersion::latest());
        if let Some(region) = region {
            builder = builder.region(Region::new(region));
        }
        builder.build()
    }

    #[tokio::test]
    async fn wif_aws_01_requests_rs256_token_from_a_regional_endpoint() {
        let stub = StubSts::new(|| Ok(Some("aws-jwt".to_owned())));

        let token = provider(&stub).token().await.unwrap();

        assert_eq!(token, "aws-jwt");
        assert_eq!(
            stub.calls(),
            vec![(AUDIENCE.to_owned(), "RS256".to_owned())]
        );

        // The client is configured for us-east-1, and for that region the AWS
        // SDK resolves the regional endpoint, not sts.amazonaws.com.
        let config = sts_config(&sdk_config(Some("us-east-1"))).unwrap();
        let region = config.region().map(Region::as_ref).unwrap();
        assert_eq!(region, "us-east-1");
        assert!(!config.retry_config().unwrap().has_retry());
        let params = Params::builder().region(region).build().unwrap();
        let endpoint = DefaultResolver::new()
            .resolve_endpoint(&params)
            .await
            .unwrap();
        assert_eq!(endpoint.url(), "https://sts.us-east-1.amazonaws.com");

        // With no region available, building the provider is an error.
        let error = sts_config(&sdk_config(None)).unwrap_err();
        assert!(
            error.to_string().contains("region"),
            "unexpected message: {error}"
        );
        // So is the pseudo-region that means "the global endpoint".
        assert!(sts_config(&sdk_config(Some("aws-global"))).is_err());
    }

    #[tokio::test]
    async fn wif_aws_02_sts_error_names_the_call_and_the_code() {
        let stub = StubSts::new(|| {
            Err(StsFailure {
                code: Some("AccessDenied".to_owned()),
                detail: "User is not authorized to perform: sts:GetWebIdentityToken".to_owned(),
            })
        });

        let error = provider(&stub).token().await.unwrap_err();

        let message = error.to_string();
        assert!(
            message.contains("sts:GetWebIdentityToken"),
            "unexpected message: {message}"
        );
        assert!(
            message.contains("AccessDenied"),
            "unexpected message: {message}"
        );
        assert!(!format!("{message} {error:?}").contains("aws-jwt"));
        assert_eq!(stub.calls().len(), 1);
    }

    /// A shared AWS configuration that sends requests to `endpoint` instead
    /// of AWS, over plain HTTP and signed with made-up credentials.
    fn sdk_config_for(endpoint: &str) -> SdkConfig {
        SdkConfig::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .endpoint_url(endpoint)
            .http_client(aws_smithy_http_client::Builder::new().build_http())
            .credentials_provider(SharedCredentialsProvider::new(Credentials::new(
                "test-access-key-id",
                "test-secret-access-key",
                None,
                None,
                "test",
            )))
            .build()
    }

    /// A provider with the real STS client, configured the way `aws_sts`
    /// configures it.
    fn provider_calling(endpoint: &str) -> AwsSts {
        let config = sts_config(&sdk_config_for(endpoint)).unwrap();
        AwsSts {
            audience: AUDIENCE.to_owned(),
            sts: Arc::new(aws_sdk_sts::Client::from_conf(config)),
        }
    }

    /// An STS error response with the given status and AWS error code.
    fn sts_error(status: u16, code: &str) -> ResponseTemplate {
        let body = format!(
            "<ErrorResponse xmlns=\"https://sts.amazonaws.com/doc/2011-06-15/\">\
             <Error><Type>Sender</Type><Code>{code}</Code><Message>try again later</Message></Error>\
             <RequestId>00000000-0000-0000-0000-000000000000</RequestId>\
             </ErrorResponse>"
        );
        ResponseTemplate::new(status).set_body_raw(body, "text/xml")
    }

    #[tokio::test]
    async fn one_provider_call_is_one_sts_request_when_sts_throttles_or_fails() {
        // Both are answers the AWS SDK retries by default.
        for (status, code) in [(400, "Throttling"), (503, "ServiceUnavailable")] {
            let sts = MockServer::start().await;
            Mock::given(any())
                .respond_with(sts_error(status, code))
                .mount(&sts)
                .await;

            let error = provider_calling(&sts.uri()).token().await.unwrap_err();

            assert!(
                error
                    .to_string()
                    .contains(&format!("sts:GetWebIdentityToken failed ({code})")),
                "unexpected message: {error}"
            );
            assert_eq!(sts.received_requests().await.unwrap().len(), 1);
        }
    }

    #[test]
    fn the_sts_client_has_ten_second_connect_and_request_limits() {
        // The shared configuration `aws_sts` loads brings the AWS SDK's
        // defaults: a 3.1-second connect limit and no limit on the request.
        let shared = SdkConfig::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .timeout_config(
                TimeoutConfig::builder()
                    .connect_timeout(Duration::from_millis(3100))
                    .build(),
            )
            .build();

        for shared in [shared, sdk_config(Some("us-east-1"))] {
            let config = sts_config(&shared).unwrap();

            let timeouts = config.timeout_config().unwrap();
            assert_eq!(timeouts.connect_timeout(), Some(Duration::from_secs(10)));
            assert_eq!(timeouts.operation_timeout(), Some(Duration::from_secs(10)));
        }
    }

    #[tokio::test]
    async fn an_empty_token_is_an_error() {
        for answer in [|| Ok(None), || Ok(Some(String::new()))] as [fn() -> _; 2] {
            let stub = StubSts::new(answer);
            let error = provider(&stub).token().await.unwrap_err();
            assert!(error.to_string().contains("sts:GetWebIdentityToken"));
        }
    }
}
