use std::sync::{Arc, Mutex, MutexGuard};

use etebase::managers;

use crate::{
    error::Result,
    lock,
    types::{
        FetchOptions, ItemListResponse, ItemMetadata, ItemRevisionsListResponse, with_fetch_options,
    },
};

/// Creates, fetches, lists and uploads the items of one collection.
#[derive(uniffi::Object)]
pub struct ItemManager {
    pub(crate) inner: managers::ItemManager,
}

impl ItemManager {
    pub(crate) fn wrap(inner: managers::ItemManager) -> Arc<Self> {
        Arc::new(Self { inner })
    }
}

/// Locks each distinct item of `lists` once and passes `f` shared references in list order.
///
/// Callers may pass the same item several times, within one list or across lists (e.g. as an item
/// and a dependency). Locking it once avoids self-deadlock; locking in address order avoids
/// deadlocks between concurrent calls. The etebase methods only take shared references, so
/// repeated items alias immutably; state they update through `&Item` (the saved etag) lands in
/// the original item, which Kotlin holds.
fn with_locked_items<const N: usize, R>(
    lists: [&[Arc<Item>]; N],
    f: impl FnOnce([Vec<&etebase::Item>; N]) -> R,
) -> R {
    let address = |item: &Item| std::ptr::from_ref(item).addr();
    let mut unique: Vec<&Item> = lists
        .iter()
        .flat_map(|list| list.iter().map(|x| &**x))
        .collect();
    unique.sort_unstable_by_key(|item| address(item));
    unique.dedup_by_key(|item| address(item));
    let guards: Vec<MutexGuard<'_, etebase::Item>> =
        unique.iter().map(|item| lock(&item.inner)).collect();
    let locked = lists.map(|list| {
        list.iter()
            .map(|item| {
                let index = unique
                    .binary_search_by_key(&address(item), |x| address(x))
                    .expect("every listed item was locked");
                &*guards[index]
            })
            .collect()
    });
    f(locked)
}

#[uniffi::export]
impl ItemManager {
    #[uniffi::method(default(fetch_options = None))]
    pub fn fetch(
        &self,
        item_uid: String,
        fetch_options: Option<FetchOptions>,
    ) -> Result<Arc<Item>> {
        with_fetch_options(fetch_options, |o| self.inner.fetch(&item_uid, o)).map(Item::wrap)
    }

    /// Creates a new local item; upload it (e.g. with `batch`) to save it on the server.
    pub fn create(&self, meta: ItemMetadata, content: Vec<u8>) -> Result<Arc<Item>> {
        let meta = etebase::ItemMetadata::from(meta);
        Ok(Item::wrap(self.inner.create(&meta, &content)?))
    }

    /// Like [`ItemManager::create`], with already serialized metadata.
    pub fn create_raw(&self, meta: Vec<u8>, content: Vec<u8>) -> Result<Arc<Item>> {
        Ok(Item::wrap(self.inner.create_raw(&meta, &content)?))
    }

    #[uniffi::method(default(fetch_options = None))]
    pub fn list(&self, fetch_options: Option<FetchOptions>) -> Result<ItemListResponse> {
        with_fetch_options(fetch_options, |o| self.inner.list(o)).map(Into::into)
    }

    #[uniffi::method(default(fetch_options = None))]
    pub fn item_revisions(
        &self,
        item: &Item,
        fetch_options: Option<FetchOptions>,
    ) -> Result<ItemRevisionsListResponse> {
        let item = lock(&item.inner);
        with_fetch_options(fetch_options, |o| self.inner.item_revisions(&item, o)).map(Into::into)
    }

    /// Fetches the server versions of those `items` that changed there.
    #[uniffi::method(default(fetch_options = None))]
    pub fn fetch_updates(
        &self,
        items: Vec<Arc<Item>>,
        fetch_options: Option<FetchOptions>,
    ) -> Result<ItemListResponse> {
        with_locked_items([&items], |[items]| {
            with_fetch_options(fetch_options, |o| {
                self.inner.fetch_updates(items.into_iter(), o)
            })
        })
        .map(Into::into)
    }

    /// Uploads `items`, overwriting concurrent changes on the server. Fails with `Conflict` if
    /// any of `deps` changed on the server.
    #[uniffi::method(default(deps = None, fetch_options = None))]
    pub fn batch(
        &self,
        items: Vec<Arc<Item>>,
        deps: Option<Vec<Arc<Item>>>,
        fetch_options: Option<FetchOptions>,
    ) -> Result<()> {
        let deps = deps.unwrap_or_default();
        with_locked_items([&items, &deps], |[items, deps]| {
            with_fetch_options(fetch_options, |o| {
                self.inner
                    .batch_deps(items.into_iter(), deps.into_iter(), o)
            })
        })
    }

    /// Uploads `items`; fails with `Conflict` if any of `items` or `deps` changed on the server.
    #[uniffi::method(default(deps = None, fetch_options = None))]
    pub fn transaction(
        &self,
        items: Vec<Arc<Item>>,
        deps: Option<Vec<Arc<Item>>>,
        fetch_options: Option<FetchOptions>,
    ) -> Result<()> {
        let deps = deps.unwrap_or_default();
        with_locked_items([&items, &deps], |[items, deps]| {
            with_fetch_options(fetch_options, |o| {
                self.inner
                    .transaction_deps(items.into_iter(), deps.into_iter(), o)
            })
        })
    }

    /// Downloads the content of an item fetched without it (see `isMissingContent`).
    pub fn download_content(&self, item: &Item) -> Result<()> {
        Ok(self.inner.download_content(&mut lock(&item.inner))?)
    }

    /// Uploads the content of an item, so a later batch need not include it.
    pub fn upload_content(&self, item: &Item) -> Result<()> {
        Ok(self.inner.upload_content(&lock(&item.inner))?)
    }

    pub fn cache_load(&self, cached: Vec<u8>) -> Result<Arc<Item>> {
        Ok(Item::wrap(self.inner.cache_load(&cached)?))
    }

    pub fn cache_save(&self, item: &Item) -> Result<Vec<u8>> {
        Ok(self.inner.cache_save(&lock(&item.inner))?)
    }

    pub fn cache_save_with_content(&self, item: &Item) -> Result<Vec<u8>> {
        Ok(self.inner.cache_save_with_content(&lock(&item.inner))?)
    }
}

/// An encrypted item.
#[derive(uniffi::Object)]
pub struct Item {
    pub(crate) inner: Mutex<etebase::Item>,
}

impl Item {
    pub(crate) fn wrap(inner: etebase::Item) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(inner),
        })
    }
}

#[uniffi::export]
impl Item {
    /// Verifies the integrity of the item; false if it was tampered with.
    pub fn verify(&self) -> bool {
        lock(&self.inner).verify().unwrap_or(false)
    }

    pub fn set_meta(&self, meta: ItemMetadata) -> Result<()> {
        Ok(lock(&self.inner).set_meta(&etebase::ItemMetadata::from(meta))?)
    }

    pub fn meta(&self) -> Result<ItemMetadata> {
        Ok(lock(&self.inner).meta()?.into())
    }

    pub fn set_meta_raw(&self, meta: Vec<u8>) -> Result<()> {
        Ok(lock(&self.inner).set_meta_raw(&meta)?)
    }

    pub fn meta_raw(&self) -> Result<Vec<u8>> {
        Ok(lock(&self.inner).meta_raw()?)
    }

    pub fn set_content(&self, content: Vec<u8>) -> Result<()> {
        Ok(lock(&self.inner).set_content(&content)?)
    }

    pub fn content(&self) -> Result<Vec<u8>> {
        Ok(lock(&self.inner).content()?)
    }

    /// Marks the item as deleted (a sync tombstone); upload it to delete it on the server.
    pub fn mark_deleted(&self) -> Result<()> {
        Ok(lock(&self.inner).delete()?)
    }

    pub fn is_deleted(&self) -> bool {
        lock(&self.inner).is_deleted()
    }

    /// Whether the item was fetched without its content (see `ItemManager.downloadContent`).
    pub fn is_missing_content(&self) -> bool {
        lock(&self.inner).is_missing_content()
    }

    pub fn uid(&self) -> String {
        lock(&self.inner).uid().to_owned()
    }

    /// The uid of the current revision.
    pub fn etag(&self) -> String {
        lock(&self.inner).etag().to_owned()
    }
}
