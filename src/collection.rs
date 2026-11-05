use std::sync::{Arc, Mutex};

use etebase::managers;

use crate::{
    error::Result,
    item::{Item, ItemManager},
    lock,
    types::{
        CollectionAccessLevel, CollectionListResponse, FetchOptions, InvitationListResponse,
        ItemMetadata, MemberListResponse, UserProfile, with_fetch_options,
    },
};

/// Creates, fetches, lists and uploads collections.
#[derive(uniffi::Object)]
pub struct CollectionManager {
    pub(crate) inner: managers::CollectionManager,
}

impl CollectionManager {
    pub(crate) fn wrap(inner: managers::CollectionManager) -> Arc<Self> {
        Arc::new(Self { inner })
    }
}

#[uniffi::export]
impl CollectionManager {
    #[uniffi::method(default(fetch_options = None))]
    pub fn fetch(
        &self,
        col_uid: String,
        fetch_options: Option<FetchOptions>,
    ) -> Result<Arc<Collection>> {
        with_fetch_options(fetch_options, |o| self.inner.fetch(&col_uid, o)).map(Collection::wrap)
    }

    /// Creates a new local collection; upload it to save it on the server.
    pub fn create(
        &self,
        collection_type: String,
        meta: ItemMetadata,
        content: Vec<u8>,
    ) -> Result<Arc<Collection>> {
        let meta = etebase::ItemMetadata::from(meta);
        Ok(Collection::wrap(self.inner.create(
            &collection_type,
            &meta,
            &content,
        )?))
    }

    /// Like [`CollectionManager::create`], with already serialized metadata.
    pub fn create_raw(
        &self,
        collection_type: String,
        meta: Vec<u8>,
        content: Vec<u8>,
    ) -> Result<Arc<Collection>> {
        Ok(Collection::wrap(self.inner.create_raw(
            &collection_type,
            &meta,
            &content,
        )?))
    }

    pub fn item_manager(&self, col: &Collection) -> Result<Arc<ItemManager>> {
        Ok(ItemManager::wrap(
            self.inner.item_manager(&lock(&col.inner))?,
        ))
    }

    pub fn member_manager(&self, col: &Collection) -> Result<Arc<CollectionMemberManager>> {
        Ok(Arc::new(CollectionMemberManager {
            inner: self.inner.member_manager(&lock(&col.inner))?,
        }))
    }

    #[uniffi::method(default(fetch_options = None))]
    pub fn list(
        &self,
        collection_type: String,
        fetch_options: Option<FetchOptions>,
    ) -> Result<CollectionListResponse> {
        with_fetch_options(fetch_options, |o| self.inner.list(&collection_type, o)).map(Into::into)
    }

    /// Lists the collections of any of `collectionTypes`.
    #[uniffi::method(default(fetch_options = None))]
    pub fn list_multi(
        &self,
        collection_types: Vec<String>,
        fetch_options: Option<FetchOptions>,
    ) -> Result<CollectionListResponse> {
        with_fetch_options(fetch_options, |o| {
            self.inner.list_multi(&collection_types, o)
        })
        .map(Into::into)
    }

    /// Uploads the collection, overwriting concurrent changes on the server.
    #[uniffi::method(default(fetch_options = None))]
    pub fn upload(
        &self,
        collection: &Collection,
        fetch_options: Option<FetchOptions>,
    ) -> Result<()> {
        let collection = lock(&collection.inner);
        with_fetch_options(fetch_options, |o| self.inner.upload(&collection, o))
    }

    /// Uploads the collection; fails with `Conflict` if it changed on the server.
    #[uniffi::method(default(fetch_options = None))]
    pub fn transaction(
        &self,
        collection: &Collection,
        fetch_options: Option<FetchOptions>,
    ) -> Result<()> {
        let collection = lock(&collection.inner);
        with_fetch_options(fetch_options, |o| self.inner.transaction(&collection, o))
    }

    pub fn cache_load(&self, cached: Vec<u8>) -> Result<Arc<Collection>> {
        Ok(Collection::wrap(self.inner.cache_load(&cached)?))
    }

    pub fn cache_save(&self, collection: &Collection) -> Result<Vec<u8>> {
        Ok(self.inner.cache_save(&lock(&collection.inner))?)
    }

    pub fn cache_save_with_content(&self, collection: &Collection) -> Result<Vec<u8>> {
        Ok(self
            .inner
            .cache_save_with_content(&lock(&collection.inner))?)
    }
}

/// An encrypted collection of items.
#[derive(uniffi::Object)]
pub struct Collection {
    pub(crate) inner: Mutex<etebase::Collection>,
}

impl Collection {
    pub(crate) fn wrap(inner: etebase::Collection) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(inner),
        })
    }
}

#[uniffi::export]
impl Collection {
    /// Verifies the integrity of the collection; false if it was tampered with.
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

    /// Marks the collection as deleted (a sync tombstone); upload it to delete it on the server.
    pub fn mark_deleted(&self) -> Result<()> {
        Ok(lock(&self.inner).delete()?)
    }

    pub fn is_deleted(&self) -> bool {
        lock(&self.inner).is_deleted()
    }

    pub fn uid(&self) -> String {
        lock(&self.inner).uid().to_owned()
    }

    pub fn etag(&self) -> String {
        lock(&self.inner).etag().to_owned()
    }

    /// The sync token of the collection's items.
    pub fn stoken(&self) -> Option<String> {
        lock(&self.inner).stoken().map(str::to_owned)
    }

    /// A copy of the collection as an item, e.g. to use as a batch dependency.
    pub fn as_item(&self) -> Result<Arc<Item>> {
        Ok(Item::wrap(lock(&self.inner).item()?))
    }

    pub fn access_level(&self) -> CollectionAccessLevel {
        lock(&self.inner).access_level().into()
    }

    pub fn collection_type(&self) -> Result<String> {
        Ok(lock(&self.inner).collection_type()?)
    }
}

/// Manages the invitations of the account.
#[derive(uniffi::Object)]
pub struct CollectionInvitationManager {
    inner: managers::CollectionInvitationManager,
}

impl CollectionInvitationManager {
    pub(crate) fn wrap(inner: managers::CollectionInvitationManager) -> Arc<Self> {
        Arc::new(Self { inner })
    }
}

#[uniffi::export]
impl CollectionInvitationManager {
    #[uniffi::method(default(fetch_options = None))]
    pub fn list_incoming(
        &self,
        fetch_options: Option<FetchOptions>,
    ) -> Result<InvitationListResponse> {
        with_fetch_options(fetch_options, |o| self.inner.list_incoming(o)).map(Into::into)
    }

    #[uniffi::method(default(fetch_options = None))]
    pub fn list_outgoing(
        &self,
        fetch_options: Option<FetchOptions>,
    ) -> Result<InvitationListResponse> {
        with_fetch_options(fetch_options, |o| self.inner.list_outgoing(o)).map(Into::into)
    }

    pub fn accept(&self, invitation: &SignedInvitation) -> Result<()> {
        Ok(self.inner.accept(&invitation.inner)?)
    }

    pub fn reject(&self, invitation: &SignedInvitation) -> Result<()> {
        Ok(self.inner.reject(&invitation.inner)?)
    }

    pub fn fetch_user_profile(&self, username: String) -> Result<UserProfile> {
        let profile = self.inner.fetch_user_profile(&username)?;
        Ok(UserProfile {
            pubkey: profile.pubkey().to_vec(),
        })
    }

    /// Invites `username`, whose public key `pubkey` was verified out of band.
    pub fn invite(
        &self,
        collection: &Collection,
        username: String,
        pubkey: Vec<u8>,
        access_level: CollectionAccessLevel,
    ) -> Result<()> {
        Ok(self.inner.invite(
            &lock(&collection.inner),
            &username,
            &pubkey,
            access_level.into(),
        )?)
    }

    pub fn disinvite(&self, invitation: &SignedInvitation) -> Result<()> {
        Ok(self.inner.disinvite(&invitation.inner)?)
    }

    /// The account's public key, to share with inviters.
    pub fn pubkey(&self) -> Vec<u8> {
        self.inner.pubkey().to_vec()
    }
}

/// An invitation to a collection.
#[derive(uniffi::Object)]
pub struct SignedInvitation {
    inner: etebase::SignedInvitation,
}

impl SignedInvitation {
    pub(crate) fn wrap(inner: etebase::SignedInvitation) -> Arc<Self> {
        Arc::new(Self { inner })
    }
}

#[uniffi::export]
impl SignedInvitation {
    pub fn uid(&self) -> String {
        self.inner.uid().to_owned()
    }

    /// The invited user.
    pub fn username(&self) -> String {
        self.inner.username().to_owned()
    }

    /// The uid of the collection.
    pub fn collection(&self) -> String {
        self.inner.collection().to_owned()
    }

    pub fn access_level(&self) -> CollectionAccessLevel {
        self.inner.access_level().into()
    }

    pub fn sender_username(&self) -> Option<String> {
        self.inner.sender_username().map(str::to_owned)
    }

    pub fn sender_pubkey(&self) -> Vec<u8> {
        self.inner.sender_pubkey().to_vec()
    }
}

/// Manages the members of a collection.
#[derive(uniffi::Object)]
pub struct CollectionMemberManager {
    inner: managers::CollectionMemberManager,
}

#[uniffi::export]
impl CollectionMemberManager {
    #[uniffi::method(default(fetch_options = None))]
    pub fn list(&self, fetch_options: Option<FetchOptions>) -> Result<MemberListResponse> {
        with_fetch_options(fetch_options, |o| self.inner.list(o)).map(Into::into)
    }

    pub fn remove(&self, username: String) -> Result<()> {
        Ok(self.inner.remove(&username)?)
    }

    /// Leaves the collection.
    pub fn leave(&self) -> Result<()> {
        Ok(self.inner.leave()?)
    }

    pub fn modify_access_level(
        &self,
        username: String,
        access_level: CollectionAccessLevel,
    ) -> Result<()> {
        Ok(self
            .inner
            .modify_access_level(&username, access_level.into())?)
    }
}
