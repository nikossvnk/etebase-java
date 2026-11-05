use std::{path::Path, sync::Arc};

use crate::{
    account::Account,
    client::Client,
    collection::{Collection, CollectionManager},
    error::Result,
    item::{Item, ItemManager},
    lock,
};

/// An on-disk cache of an account's session, collections, items and sync tokens.
///
/// The format is shared with the other Etebase bindings and with earlier versions of this library.
#[derive(uniffi::Object)]
pub struct FileSystemCache {
    inner: etebase::fs_cache::FileSystemCache,
}

#[uniffi::export]
impl FileSystemCache {
    /// Opens (creating if needed) the cache of `username` below the directory `path`.
    #[uniffi::constructor]
    pub fn create(path: String, username: String) -> Result<Arc<Self>> {
        Ok(Arc::new(Self {
            inner: etebase::fs_cache::FileSystemCache::new(Path::new(&path), &username)?,
        }))
    }

    /// Deletes everything cached for this user.
    pub fn clear_user_cache(&self) -> Result<()> {
        Ok(self.inner.clear_user_cache()?)
    }

    #[uniffi::method(default(encryption_key = None))]
    pub fn account_save(&self, account: &Account, encryption_key: Option<Vec<u8>>) -> Result<()> {
        account.with(|account| {
            Ok(self
                .inner
                .save_account(account, encryption_key.as_deref())?)
        })
    }

    #[uniffi::method(default(encryption_key = None))]
    pub fn account_load(
        &self,
        client: &Client,
        encryption_key: Option<Vec<u8>>,
    ) -> Result<Arc<Account>> {
        let account = self
            .inner
            .load_account(&client.snapshot(), encryption_key.as_deref())?;
        Ok(Account::wrap(account))
    }

    /// Saves the sync token of the collection list.
    pub fn save_stoken(&self, stoken: String) -> Result<()> {
        Ok(self.inner.save_stoken(&stoken)?)
    }

    pub fn load_stoken(&self) -> Result<Option<String>> {
        Ok(self.inner.load_stoken()?)
    }

    /// Saves the sync token of the items of collection `colUid`.
    pub fn collection_save_stoken(&self, col_uid: String, stoken: String) -> Result<()> {
        Ok(self.inner.collection_save_stoken(&col_uid, &stoken)?)
    }

    pub fn collection_load_stoken(&self, col_uid: String) -> Result<Option<String>> {
        Ok(self.inner.collection_load_stoken(&col_uid)?)
    }

    pub fn collection_get(
        &self,
        col_mgr: &CollectionManager,
        col_uid: String,
    ) -> Result<Arc<Collection>> {
        Ok(Collection::wrap(
            self.inner.collection(&col_mgr.inner, &col_uid)?,
        ))
    }

    pub fn collection_set(
        &self,
        col_mgr: &CollectionManager,
        collection: &Collection,
    ) -> Result<()> {
        Ok(self
            .inner
            .collection_set(&col_mgr.inner, &lock(&collection.inner))?)
    }

    pub fn collection_set_with_content(
        &self,
        col_mgr: &CollectionManager,
        collection: &Collection,
    ) -> Result<()> {
        Ok(self
            .inner
            .collection_set_with_content(&col_mgr.inner, &lock(&collection.inner))?)
    }

    /// Removes the collection and its items from the cache.
    pub fn collection_unset(&self, col_mgr: &CollectionManager, col_uid: String) -> Result<()> {
        Ok(self.inner.collection_unset(&col_mgr.inner, &col_uid)?)
    }

    /// Loads every cached collection.
    pub fn collection_list(&self, col_mgr: &CollectionManager) -> Result<Vec<Arc<Collection>>> {
        let mut collections = Vec::new();
        for cached in self.inner.collection_list_raw()? {
            collections.push(Collection::wrap(col_mgr.inner.cache_load(&cached?)?));
        }
        Ok(collections)
    }

    pub fn item_get(
        &self,
        item_mgr: &ItemManager,
        col_uid: String,
        item_uid: String,
    ) -> Result<Arc<Item>> {
        Ok(Item::wrap(self.inner.item(
            &item_mgr.inner,
            &col_uid,
            &item_uid,
        )?))
    }

    pub fn item_set(&self, item_mgr: &ItemManager, col_uid: String, item: &Item) -> Result<()> {
        Ok(self
            .inner
            .item_set(&item_mgr.inner, &col_uid, &lock(&item.inner))?)
    }

    pub fn item_set_with_content(
        &self,
        item_mgr: &ItemManager,
        col_uid: String,
        item: &Item,
    ) -> Result<()> {
        Ok(self
            .inner
            .item_set_with_content(&item_mgr.inner, &col_uid, &lock(&item.inner))?)
    }

    pub fn item_unset(
        &self,
        item_mgr: &ItemManager,
        col_uid: String,
        item_uid: String,
    ) -> Result<()> {
        Ok(self
            .inner
            .item_unset(&item_mgr.inner, &col_uid, &item_uid)?)
    }

    /// Loads every cached item of collection `colUid`.
    pub fn item_list(&self, item_mgr: &ItemManager, col_uid: String) -> Result<Vec<Arc<Item>>> {
        let mut items = Vec::new();
        for cached in self.inner.item_list_raw(&col_uid)? {
            items.push(Item::wrap(item_mgr.inner.cache_load(&cached?)?));
        }
        Ok(items)
    }
}
