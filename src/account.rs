use std::sync::{Arc, Mutex};

use crate::{
    client::Client,
    collection::{CollectionInvitationManager, CollectionManager},
    error::Result,
    lock,
    types::User,
};

/// An authenticated Etebase account.
#[derive(uniffi::Object)]
pub struct Account {
    inner: Mutex<etebase::Account>,
}

impl Account {
    pub(crate) fn wrap(inner: etebase::Account) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(inner),
        })
    }

    pub(crate) fn with<R>(&self, f: impl FnOnce(&etebase::Account) -> R) -> R {
        f(&lock(&self.inner))
    }
}

#[uniffi::export]
impl Account {
    /// Logs in with a password; initializes the account on the server if needed.
    #[uniffi::constructor]
    pub fn login(client: &Client, username: String, password: String) -> Result<Arc<Self>> {
        Ok(Self::wrap(etebase::Account::login(
            client.snapshot(),
            &username,
            &password,
        )?))
    }

    /// Creates a new user on the server, authenticated by `password`.
    #[uniffi::constructor]
    pub fn signup(client: &Client, user: &User, password: String) -> Result<Arc<Self>> {
        Ok(Self::wrap(etebase::Account::signup(
            client.snapshot(),
            &user.snapshot(),
            &password,
        )?))
    }

    /// Logs in with a 32-byte main key instead of a password.
    #[uniffi::constructor]
    pub fn login_key(client: &Client, username: String, main_key: Vec<u8>) -> Result<Arc<Self>> {
        Ok(Self::wrap(etebase::Account::login_key(
            client.snapshot(),
            &username,
            &main_key,
        )?))
    }

    /// Creates a new user on the server, authenticated by a 32-byte main key.
    #[uniffi::constructor]
    pub fn signup_key(client: &Client, user: &User, main_key: Vec<u8>) -> Result<Arc<Self>> {
        Ok(Self::wrap(etebase::Account::signup_key(
            client.snapshot(),
            &user.snapshot(),
            &main_key,
        )?))
    }

    /// Restores an account saved by [`Account::save`] with the same `encryptionKey`.
    #[uniffi::constructor(default(encryption_key = None))]
    pub fn restore(
        client: &Client,
        account_data_stored: String,
        encryption_key: Option<Vec<u8>>,
    ) -> Result<Arc<Self>> {
        Ok(Self::wrap(etebase::Account::restore(
            client.snapshot(),
            &account_data_stored,
            encryption_key.as_deref(),
        )?))
    }

    /// Fetches a new authentication token, e.g. after the old one expired.
    pub fn fetch_token(&self) -> Result<()> {
        Ok(lock(&self.inner).fetch_token()?)
    }

    /// Changes the server URL used by this account.
    pub fn force_server_url(&self, api_base: String) -> Result<()> {
        Ok(lock(&self.inner).force_server_url(&api_base)?)
    }

    pub fn change_password(&self, password: String) -> Result<()> {
        Ok(lock(&self.inner).change_password(&password)?)
    }

    pub fn fetch_dashboard_url(&self) -> Result<String> {
        Ok(lock(&self.inner).fetch_dashboard_url()?)
    }

    /// Invalidates the authentication token on the server.
    pub fn logout(&self) -> Result<()> {
        Ok(lock(&self.inner).logout()?)
    }

    pub fn collection_manager(&self) -> Result<Arc<CollectionManager>> {
        Ok(CollectionManager::wrap(
            lock(&self.inner).collection_manager()?,
        ))
    }

    pub fn invitation_manager(&self) -> Result<Arc<CollectionInvitationManager>> {
        Ok(CollectionInvitationManager::wrap(
            lock(&self.inner).invitation_manager()?,
        ))
    }

    /// Serializes the session, encrypted with the optional 32-byte `encryptionKey`.
    #[uniffi::method(default(encryption_key = None))]
    pub fn save(&self, encryption_key: Option<Vec<u8>>) -> Result<String> {
        Ok(lock(&self.inner).save(encryption_key.as_deref())?)
    }
}
