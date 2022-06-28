use std::fmt::Write;

use serde::ser;
use serde::Serializer;

/// Serializes a `Vec<T>` for use within a query string.
///
/// # Examples
///
/// ```
/// #[derive(Debug, Serialize)]
/// struct List<'a> {
///     #[serde(rename = "items[]", serialize_with = "super::serialize_vec_to_query")]
///     pub items: Vec<&'a str>,
/// }
/// ```
pub(crate) fn serialize_vec_to_query<
    T: std::fmt::Debug + std::fmt::Display + Into<String>,
    S: Serializer,
>(
    value: &Vec<T>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let value = dbg!(value);

    let mut serialized = String::new();

    for (index, item) in value.iter().enumerate() {
        write!(&mut serialized, "{}", item)
            .map_err(|err| ser::Error::custom(format!("failed to write '{}': {}", item, err)))?;

        if index < value.len() - 1 {
            write!(&mut serialized, ",")
                .map_err(|err| ser::Error::custom(format!("failed to write separator: {}", err)))?
        }
    }

    serializer.serialize_str(&serialized)
}

#[cfg(test)]
mod test {
    use mockito::{self, mock, Matcher};
    use reqwest::StatusCode;
    use serde::Serialize;

    #[tokio::test]
    async fn it_serializes_a_vec_in_the_query_string() {
        #[derive(Debug, Serialize)]
        struct List<'a> {
            #[serde(rename = "items[]", serialize_with = "super::serialize_vec_to_query")]
            pub items: Vec<&'a str>,
        }

        let _mock = mock("GET", "/")
            .match_query(Matcher::UrlEncoded(
                "items[]".to_string(),
                "one,two,three".to_string(),
            ))
            .with_status(200)
            .create();

        let client = reqwest::Client::new();

        let response = client
            .get(&mockito::server_url())
            .query(&List {
                items: vec!["one", "two", "three"],
            })
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK)
    }
}
