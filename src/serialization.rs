use serde::ser;
use serde::{Serialize, Serializer};

pub(crate) fn serialize_vec<T: ?Sized + Serialize, S: Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match serde_json::to_string(value) {
        Ok(json) => serializer.serialize_str(&json),
        Err(_) => Err(ser::Error::custom("Failed to serialize &T to JSON")),
    }
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
            #[serde(serialize_with = "super::serialize_vec")]
            pub items: Vec<&'a str>,
        }

        let x = dbg!(serde_json::to_string(&List {
            items: vec!["one", "two", "three"],
        }));

        let _mock = mock("GET", "/")
            .match_query(Matcher::UrlEncoded(
                "items".to_string(),
                r#""["one","two","three"]"#.to_string(),
            ))
            .with_status(200);

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
