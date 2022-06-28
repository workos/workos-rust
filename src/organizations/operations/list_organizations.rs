use async_trait::async_trait;
use serde::Serialize;

use crate::organizations::{Organization, Organizations};
use crate::{PaginatedList, PaginationOptions, WorkOsResult};

//pub struct domainFilters()

#[derive(Debug, Serialize)]

pub struct ListOrganizationsOptions<'a> {
    /// The pagination options to use when listing organizations.
    #[serde(flatten)]
    pub pagination: PaginationOptions<'a>,
    pub domains: Option<Vec<&'a str>>,
}

impl<'a> Default for ListOrganizationsOptions<'a> {
    fn default() -> Self {
        Self {
            pagination: PaginationOptions::default(),
            domains: None,
        }
    }
}

#[async_trait]
pub trait ListOrganizations {
    /// Retrieves a list of [`Organization`]s.
    ///
    /// [WorkOS Docs: List Organizations](https://workos.com/docs/reference/organization/list)
    async fn list_organizations(
        &self,
        options: &ListOrganizationsOptions<'_>,
    ) -> WorkOsResult<PaginatedList<Organization>, ()>;
}

#[async_trait]
impl<'a> ListOrganizations for Organizations<'a> {
    async fn list_organizations(
        &self,
        options: &ListOrganizationsOptions<'_>,
    ) -> WorkOsResult<PaginatedList<Organization>, ()> {
        let url = self.workos.base_url().join("/organizations")?;
        let response = self
            .workos
            .client()
            .get(url)
            .query(&options)
            .bearer_auth(self.workos.key())
            .send()
            .await?;
        let list_organizations_response = response.json::<PaginatedList<Organization>>().await?;

        Ok(list_organizations_response)
    }
}

#[cfg(test)]
mod test {
    use mockito::{self, mock, Matcher};
    use serde_json::json;
    use tokio;

    use crate::{organizations::OrganizationId, ApiKey, WorkOs};

    use super::*;

    #[tokio::test]
    async fn it_calls_the_list_organizations_endpoint() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("GET", "/organizations")
            .match_query(Matcher::UrlEncoded("order".to_string(), "desc".to_string()))
            .match_header("Authorization", "Bearer sk_example_123456789")
            .with_status(200)
            .with_body(
                json!({
                  "data": [
                    {
                      "id": "org_01EHZNVPK3SFK441A1RGBFSHRT",
                      "object": "organization",
                      "name": "Foo Corp",
                      "allow_profiles_outside_organization": false,
                      "created_at": "2021-06-25T19:07:33.155Z",
                      "updated_at": "2021-06-25T19:07:33.155Z",
                      "domains": [
                        {
                          "domain": "foo-corp.com",
                          "id": "org_domain_01EHZNVPK2QXHMVWCEDQEKY69A",
                          "object": "organization_domain"
                        },
                        {
                          "domain": "another-foo-corp-domain.com",
                          "id": "org_domain_01EHZNS0H9W90A90FV79GAB6AB",
                          "object": "organization_domain"
                        }
                      ]
                    }
                  ],
                  "list_metadata": {
                    "before": "org_01EHZNVPK3SFK441A1RGBFSHRT",
                    "after": "org_01EJBGJT2PC6638TN5Y380M40Z",
                  }
                })
                .to_string(),
            )
            .create();

        let paginated_list = workos
            .organizations()
            .list_organizations(&Default::default())
            .await
            .unwrap();

        assert_eq!(
            paginated_list.metadata.after,
            Some("org_01EJBGJT2PC6638TN5Y380M40Z".to_string())
        )
    }

    #[tokio::test]
    async fn it_calls_the_list_organizations_endpoint_with_the_domain() {
        let workos = WorkOs::builder(&ApiKey::from("sk_example_123456789"))
            .base_url(&mockito::server_url())
            .unwrap()
            .build();

        let _mock = mock("GET", "/organizations")
            .match_query(Matcher::UrlEncoded(
                "domains".to_string(),
                "OktaSAML".to_string(),
            ))
            .match_header("Authorization", "Bearer sk_example_123456789")
            .with_status(200)
            .with_body(
                json!({
                  "id": "org_01EHZNVPK3SFK441A1RGBFSHRT",
                  "object": "organization",
                  "name": "Foo Corporation",
                  "allow_profiles_outside_organization": false,
                  "created_at": "2021-06-25T19:07:33.155Z",
                  "updated_at": "2021-06-25T19:07:33.155Z",
                  "domains": [
                    {
                      "domain": "foo-corp.com",
                      "id": "org_domain_01EHZNVPK2QXHMVWCEDQEKY69A",
                      "object": "organization_domain"
                    }
                  ]
                })
                .to_string(),
            )
            .create();

        let paginated_list = workos
            .organizations()
            .list_organizations(&ListOrganizationsOptions {
                domains: Some(vec!["foo-corp.com"]),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(
            paginated_list
                .data
                .into_iter()
                .next()
                .map(|organization| organization.id),
            Some(OrganizationId::from("org_01EHZNVPK3SFK441A1RGBFSHRT"))
        )
    }
}
