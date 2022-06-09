use serde::Serialize;

/// The order in which records should be returned when paginating.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaginationOrder {
    /// Records are returned in ascending order.
    Asc,

    /// Records are returned in descending order.
    Desc,
}

impl PaginationOrder {
    /// The default order to use for pagination.
    pub(crate) const DEFAULT: PaginationOrder = PaginationOrder::Desc;
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::PaginationOrder;

    #[test]
    fn it_properly_serializes_asc() {
        assert_eq!(
            serde_json::to_string(&PaginationOrder::Asc).unwrap(),
            json!("asc").to_string()
        )
    }

    #[test]
    fn it_properly_serializes_desc() {
        assert_eq!(
            serde_json::to_string(&PaginationOrder::Desc).unwrap(),
            json!("desc").to_string()
        )
    }
}
