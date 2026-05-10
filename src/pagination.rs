// @oagen-ignore-file
use std::future::Future;

use futures_util::stream::{self, Stream, TryStreamExt};
use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub list_metadata: ListMetadata,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
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
