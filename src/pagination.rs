// @oagen-ignore-file
//! Cursor-based pagination primitives.
//!
//! WorkOS list endpoints return a JSON envelope of the form
//! `{ "data": [...], "list_metadata": { "before": ..., "after": ... } }`.
//! The two structs in this module model that envelope; [`auto_paginate`]
//! drives a paginated endpoint to exhaustion as an async stream so callers
//! don't have to manage the cursor manually.

use std::future::Future;

use futures_util::stream::{self, Stream, TryStreamExt};
use serde::{Deserialize, Serialize};

use crate::error::Error;

/// One page of a paginated list response. `data` holds the items and
/// `list_metadata` carries the cursor used to fetch the next page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    /// The items on this page, in the order returned by the API.
    pub data: Vec<T>,
    /// Cursors for fetching the previous and next page.
    #[serde(default)]
    pub list_metadata: ListMetadata,
}

/// Cursor pair attached to every list response. Pass [`Self::after`] (or
/// [`Self::before`]) back as the `after`/`before` query parameter on the
/// next request to advance through the result set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListMetadata {
    /// Cursor for the previous page. `None` on the first page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    /// Cursor for the next page. `None` when there are no further results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

/// Drive a paginated endpoint to exhaustion as an async stream of items.
///
/// `fetch` is called with the current `after` cursor (`None` on the first
/// call). Each yielded [`Page`] contributes its `data` to the resulting
/// stream; iteration stops when `list_metadata.after` is `None`. Errors
/// short-circuit the stream.
///
/// ```ignore
/// use futures::TryStreamExt;
///
/// let stream = pagination::auto_paginate(|after| async {
///     client.things().list(ListParams { after, ..Default::default() }).await
/// });
/// let all: Vec<Thing> = stream.try_collect().await?;
/// ```
pub fn auto_paginate<T, F, Fut>(fetch: F) -> impl Stream<Item = Result<T, Error>>
where
    F: FnMut(Option<String>) -> Fut,
    Fut: Future<Output = Result<Page<T>, Error>>,
{
    let init: (Option<Option<String>>, F) = (Some(None), fetch);
    stream::try_unfold(init, |(cursor, mut fetch)| async move {
        let Some(after) = cursor else {
            return Ok::<_, Error>(None);
        };
        let page = fetch(after).await?;
        let next = page.list_metadata.after.clone();
        let next_cursor = if next.is_some() { Some(next) } else { None };
        let chunk = stream::iter(page.data.into_iter().map(Ok::<T, Error>));
        Ok(Some((chunk, (next_cursor, fetch))))
    })
    .try_flatten()
}

/// Lower-level cursor-pagination driver. Generated `*_auto_paging` methods
/// route through this helper because their endpoint-specific response types
/// don't always shape-match [`Page<T>`]; the closure decomposes a response
/// into `(items, next_after_cursor)` and the helper handles the rest.
pub(crate) fn auto_paginate_pages<T, F, Fut>(fetch: F) -> impl Stream<Item = Result<T, Error>>
where
    F: FnMut(Option<String>) -> Fut,
    Fut: Future<Output = Result<(Vec<T>, Option<String>), Error>>,
{
    let init: (Option<Option<String>>, F) = (Some(None), fetch);
    stream::try_unfold(init, |(cursor, mut fetch)| async move {
        let Some(after) = cursor else {
            return Ok::<_, Error>(None);
        };
        let (data, next_after) = fetch(after).await?;
        let next_cursor = next_after.map(Some);
        let chunk = stream::iter(data.into_iter().map(Ok::<T, Error>));
        Ok(Some((chunk, (next_cursor, fetch))))
    })
    .try_flatten()
}
