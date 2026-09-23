use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use moka::future::Cache;

use super::source;

/// In-memory cache of the source site's guessable-words dictionaries, keyed by word
/// length and first letter (every day sharing that shape shares the same list), with
/// time-to-idle eviction: an entry is dropped once it hasn't been read for `ttl`.
pub struct DictionaryCache {
    entries: Cache<(usize, char), Arc<Vec<String>>>,
}

impl DictionaryCache {
    pub fn new(ttl: Duration) -> Self {
        Self { entries: Cache::builder().time_to_idle(ttl).build() }
    }

    /// Returns the dictionary for words of `length` starting with `first_letter`,
    /// fetching it from the source site on a miss. Concurrent misses on the same key
    /// share a single fetch; a failed fetch is not cached.
    pub async fn get(
        &self,
        client: &reqwest::Client,
        length: usize,
        first_letter: char,
    ) -> Result<Arc<Vec<String>>> {
        self.entries
            .try_get_with((length, first_letter), async {
                source::fetch_possible_words(client, length, first_letter).await.map(Arc::new)
            })
            .await
            .map_err(|e| anyhow::anyhow!("{e:#}"))
    }
}
