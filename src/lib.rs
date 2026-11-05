//! UniFFI bindings of etebase-rs for Kotlin (Android and desktop JVM).
//!
//! Objects shared with Kotlin are `Send + Sync`: etebase types that are mutated or not `Sync`
//! live behind a `Mutex`. Network calls on an object hold its lock, so concurrent calls on the
//! same object run one after another.

use std::sync::{Mutex, MutexGuard, PoisonError};

mod account;
mod client;
mod collection;
mod error;
mod fs_cache;
mod item;
mod types;

pub use account::Account;
pub use client::{Client, HttpClient, HttpClientError, HttpResponse, is_etebase_server};
pub use collection::{
    Collection, CollectionInvitationManager, CollectionManager, CollectionMemberManager,
    SignedInvitation,
};
pub use error::EtebaseError;
pub use fs_cache::FileSystemCache;
pub use item::{Item, ItemManager};
pub use types::{
    CollectionAccessLevel, CollectionListResponse, CollectionMember, FetchOptions,
    InvitationListResponse, ItemListResponse, ItemMetadata, ItemRevisionsListResponse,
    MemberListResponse, PrefetchOption, RemovedCollection, User, UserProfile,
};

use error::Result;

uniffi::setup_scaffolding!();

/// Locks `mutex`. A panic while it was held was already reported to Kotlin as an
/// `InternalException`; the etebase value itself stays valid, so later calls may use it.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[uniffi::export]
pub fn from_base64(string: String) -> Result<Vec<u8>> {
    Ok(etebase::utils::from_base64(&string)?)
}

#[uniffi::export]
pub fn to_base64(bytes: Vec<u8>) -> Result<String> {
    Ok(etebase::utils::to_base64(&bytes)?)
}

/// `size` cryptographically secure random bytes.
#[uniffi::export]
pub fn random_bytes(size: u32) -> Vec<u8> {
    etebase::utils::randombytes(size as usize)
}

/// A human-readable fingerprint of a public key, for out-of-band verification.
#[uniffi::export]
pub fn pretty_fingerprint(content: Vec<u8>) -> String {
    etebase::pretty_fingerprint(&content)
}
