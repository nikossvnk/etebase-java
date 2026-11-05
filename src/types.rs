use std::sync::{Arc, Mutex};

use crate::{
    collection::{Collection, SignedInvitation},
    error::Result,
    item::Item,
    lock,
};

/// How much data list operations prefetch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum PrefetchOption {
    /// Decide automatically based on the size of the data
    Auto,
    /// Fetch a lighter (medium) amount of data
    Medium,
}

impl From<PrefetchOption> for etebase::PrefetchOption {
    fn from(value: PrefetchOption) -> Self {
        match value {
            PrefetchOption::Auto => Self::Auto,
            PrefetchOption::Medium => Self::Medium,
        }
    }
}

/// A user's access level to a collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum CollectionAccessLevel {
    ReadOnly,
    Admin,
    ReadWrite,
}

impl From<CollectionAccessLevel> for etebase::CollectionAccessLevel {
    fn from(value: CollectionAccessLevel) -> Self {
        match value {
            CollectionAccessLevel::ReadOnly => Self::ReadOnly,
            CollectionAccessLevel::Admin => Self::Admin,
            CollectionAccessLevel::ReadWrite => Self::ReadWrite,
        }
    }
}

impl From<etebase::CollectionAccessLevel> for CollectionAccessLevel {
    fn from(value: etebase::CollectionAccessLevel) -> Self {
        match value {
            etebase::CollectionAccessLevel::ReadOnly => Self::ReadOnly,
            etebase::CollectionAccessLevel::Admin => Self::Admin,
            etebase::CollectionAccessLevel::ReadWrite => Self::ReadWrite,
        }
    }
}

/// Options for fetch and list operations. Unset fields use the server defaults.
#[derive(Clone, Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct FetchOptions {
    /// Maximum number of objects to return
    #[uniffi(default)]
    pub limit: Option<u64>,
    /// Sync token: only return changes after it
    #[uniffi(default)]
    pub stoken: Option<String>,
    /// Iterator for paginated listings
    #[uniffi(default)]
    pub iterator: Option<String>,
    #[uniffi(default)]
    pub prefetch: Option<PrefetchOption>,
    /// Include the collection itself when listing items
    #[uniffi(default)]
    pub with_collection: Option<bool>,
}

/// Runs `f` with the etebase equivalent of `options`, which borrows from it.
pub(crate) fn with_fetch_options<R>(
    options: Option<FetchOptions>,
    f: impl FnOnce(Option<&etebase::FetchOptions<'_>>) -> etebase::error::Result<R>,
) -> Result<R> {
    let Some(options) = options else {
        return Ok(f(None)?);
    };
    let prefetch = options.prefetch.map(etebase::PrefetchOption::from);
    let mut converted = etebase::FetchOptions::new()
        .stoken(options.stoken.as_deref())
        .iterator(options.iterator.as_deref());
    if let Some(limit) = options.limit {
        // A limit beyond the address space (only possible on 32-bit targets) means "no limit".
        converted = converted.limit(usize::try_from(limit).unwrap_or(usize::MAX));
    }
    if let Some(prefetch) = &prefetch {
        converted = converted.prefetch(prefetch);
    }
    if let Some(with_collection) = options.with_collection {
        converted = converted.with_collection(with_collection);
    }
    Ok(f(Some(&converted))?)
}

/// Plain-text metadata of a collection or item.
#[derive(Clone, Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct ItemMetadata {
    /// The item type (serialized as `type`)
    #[uniffi(default)]
    pub item_type: Option<String>,
    #[uniffi(default)]
    pub name: Option<String>,
    /// Modification time in milliseconds since the Unix epoch
    #[uniffi(default)]
    pub mtime: Option<i64>,
    #[uniffi(default)]
    pub description: Option<String>,
    #[uniffi(default)]
    pub color: Option<String>,
}

impl From<ItemMetadata> for etebase::ItemMetadata {
    fn from(value: ItemMetadata) -> Self {
        let mut meta = Self::new();
        meta.set_item_type(value.item_type)
            .set_name(value.name)
            .set_mtime(value.mtime)
            .set_description(value.description)
            .set_color(value.color);
        meta
    }
}

impl From<etebase::ItemMetadata> for ItemMetadata {
    fn from(value: etebase::ItemMetadata) -> Self {
        Self {
            item_type: value.item_type().map(str::to_owned),
            name: value.name().map(str::to_owned),
            mtime: value.mtime(),
            description: value.description().map(str::to_owned),
            color: value.color().map(str::to_owned),
        }
    }
}

/// A collection the user no longer has access to.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct RemovedCollection {
    pub uid: String,
}

#[derive(uniffi::Record)]
pub struct CollectionListResponse {
    pub data: Vec<Arc<Collection>>,
    pub stoken: Option<String>,
    pub done: bool,
    pub removed_memberships: Vec<RemovedCollection>,
}

impl From<etebase::CollectionListResponse<etebase::Collection>> for CollectionListResponse {
    fn from(value: etebase::CollectionListResponse<etebase::Collection>) -> Self {
        Self {
            data: value.data().iter().cloned().map(Collection::wrap).collect(),
            stoken: value.stoken().map(str::to_owned),
            done: value.done(),
            removed_memberships: value
                .removed_memberships()
                .map(|removed| {
                    removed
                        .iter()
                        .map(|x| RemovedCollection {
                            uid: x.uid().to_owned(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

#[derive(uniffi::Record)]
pub struct ItemListResponse {
    pub data: Vec<Arc<Item>>,
    pub stoken: Option<String>,
    pub done: bool,
}

impl From<etebase::ItemListResponse<etebase::Item>> for ItemListResponse {
    fn from(value: etebase::ItemListResponse<etebase::Item>) -> Self {
        Self {
            data: value.data().iter().cloned().map(Item::wrap).collect(),
            stoken: value.stoken().map(str::to_owned),
            done: value.done(),
        }
    }
}

#[derive(uniffi::Record)]
pub struct ItemRevisionsListResponse {
    pub data: Vec<Arc<Item>>,
    pub iterator: Option<String>,
    pub done: bool,
}

impl From<etebase::IteratorListResponse<etebase::Item>> for ItemRevisionsListResponse {
    fn from(value: etebase::IteratorListResponse<etebase::Item>) -> Self {
        Self {
            data: value.data().iter().cloned().map(Item::wrap).collect(),
            iterator: value.iterator().map(str::to_owned),
            done: value.done(),
        }
    }
}

#[derive(uniffi::Record)]
pub struct InvitationListResponse {
    pub data: Vec<Arc<SignedInvitation>>,
    pub iterator: Option<String>,
    pub done: bool,
}

impl From<etebase::IteratorListResponse<etebase::SignedInvitation>> for InvitationListResponse {
    fn from(value: etebase::IteratorListResponse<etebase::SignedInvitation>) -> Self {
        Self {
            data: value
                .data()
                .iter()
                .cloned()
                .map(SignedInvitation::wrap)
                .collect(),
            iterator: value.iterator().map(str::to_owned),
            done: value.done(),
        }
    }
}

/// A member of a collection.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CollectionMember {
    pub username: String,
    pub access_level: CollectionAccessLevel,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct MemberListResponse {
    pub data: Vec<CollectionMember>,
    pub iterator: Option<String>,
    pub done: bool,
}

impl From<etebase::IteratorListResponse<etebase::CollectionMember>> for MemberListResponse {
    fn from(value: etebase::IteratorListResponse<etebase::CollectionMember>) -> Self {
        Self {
            data: value
                .data()
                .iter()
                .map(|member| CollectionMember {
                    username: member.username().to_owned(),
                    access_level: member.access_level().into(),
                })
                .collect(),
            iterator: value.iterator().map(str::to_owned),
            done: value.done(),
        }
    }
}

/// The public profile of a user, used to invite them to collections.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct UserProfile {
    pub pubkey: Vec<u8>,
}

/// User details for signing up.
#[derive(uniffi::Object)]
pub struct User {
    inner: Mutex<etebase::User>,
}

impl User {
    /// A copy of the user, so network calls do not hold the lock.
    pub(crate) fn snapshot(&self) -> etebase::User {
        let user = lock(&self.inner);
        etebase::User::new(user.username(), user.email())
    }
}

#[uniffi::export]
impl User {
    #[uniffi::constructor]
    pub fn new(username: String, email: String) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(etebase::User::new(&username, &email)),
        })
    }

    pub fn username(&self) -> String {
        lock(&self.inner).username().to_owned()
    }

    pub fn set_username(&self, username: String) {
        lock(&self.inner).set_username(&username);
    }

    pub fn email(&self) -> String {
        lock(&self.inner).email().to_owned()
    }

    pub fn set_email(&self, email: String) {
        lock(&self.inner).set_email(&email);
    }
}
