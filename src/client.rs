use std::sync::{Arc, Mutex};

use etebase::{
    error::Error,
    http_custom_client::{ClientImplementation, Response},
};

use crate::{error::Result, lock};

/// The raw result of an HTTP request made by an [`HttpClient`].
#[derive(uniffi::Record)]
pub struct HttpResponse {
    /// HTTP status code
    pub status: u16,
    /// Raw response body
    pub body: Vec<u8>,
}

/// An error an [`HttpClient`] may throw when a request cannot be completed.
///
/// Any other exception thrown by an `HttpClient` is handled the same way: Etebase reports it as
/// `EtebaseException.Connection`, using the exception's `toString()` as the message.
#[derive(Debug, uniffi::Error)]
pub enum HttpClientError {
    Connection { reason: String },
}

impl std::fmt::Display for HttpClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection { reason } => f.write_str(reason),
        }
    }
}

impl From<uniffi::UnexpectedUniFFICallbackError> for HttpClientError {
    fn from(err: uniffi::UnexpectedUniFFICallbackError) -> Self {
        // Kotlin reports unexpected exceptions with their stack trace; the first line is the
        // exception's `toString()`.
        let reason = err.reason.lines().next().unwrap_or_default().to_owned();
        Self::Connection { reason }
    }
}

/// The HTTP transport Etebase uses, implemented by the application (e.g. with OkHttp).
///
/// Every request sends and accepts `application/msgpack`. When `authToken` is not null, send it as
/// `Authorization: Token <authToken>`. Return the response for every HTTP status, including errors;
/// throw only if no response was received. Methods may be called from any thread.
#[uniffi::export(foreign)]
pub trait HttpClient: Send + Sync {
    fn get(&self, url: String, auth_token: Option<String>)
    -> Result<HttpResponse, HttpClientError>;
    fn post(
        &self,
        url: String,
        auth_token: Option<String>,
        body: Vec<u8>,
    ) -> Result<HttpResponse, HttpClientError>;
    fn put(
        &self,
        url: String,
        auth_token: Option<String>,
        body: Vec<u8>,
    ) -> Result<HttpResponse, HttpClientError>;
    fn patch(
        &self,
        url: String,
        auth_token: Option<String>,
        body: Vec<u8>,
    ) -> Result<HttpResponse, HttpClientError>;
    fn delete(
        &self,
        url: String,
        auth_token: Option<String>,
    ) -> Result<HttpResponse, HttpClientError>;
}

struct ForeignHttpClient(Arc<dyn HttpClient>);

fn response(result: Result<HttpResponse, HttpClientError>) -> Response {
    match result {
        Ok(HttpResponse { status, body }) => Response::new(body, status),
        Err(err) => Response::new_err(Error::Connection(err.to_string())),
    }
}

fn token(auth_token: Option<&str>) -> Option<String> {
    auth_token.map(str::to_owned)
}

impl ClientImplementation for ForeignHttpClient {
    fn get(&self, url: &str, auth_token: Option<&str>) -> Response {
        response(self.0.get(url.to_owned(), token(auth_token)))
    }

    fn post(&self, url: &str, auth_token: Option<&str>, body: Vec<u8>) -> Response {
        response(self.0.post(url.to_owned(), token(auth_token), body))
    }

    fn put(&self, url: &str, auth_token: Option<&str>, body: Vec<u8>) -> Response {
        response(self.0.put(url.to_owned(), token(auth_token), body))
    }

    fn patch(&self, url: &str, auth_token: Option<&str>, body: Vec<u8>) -> Response {
        response(self.0.patch(url.to_owned(), token(auth_token), body))
    }

    fn delete(&self, url: &str, auth_token: Option<&str>) -> Response {
        response(self.0.delete(url.to_owned(), token(auth_token)))
    }
}

/// A connection to an Etebase server.
#[derive(uniffi::Object)]
pub struct Client {
    inner: Mutex<etebase::Client>,
}

impl Client {
    /// A copy of the client, so network calls do not hold the lock.
    pub(crate) fn snapshot(&self) -> etebase::Client {
        lock(&self.inner).clone()
    }
}

#[uniffi::export]
impl Client {
    /// Connects to `serverUrl` (the default Etebase server when null) through `httpClient`.
    #[uniffi::constructor]
    pub fn new(server_url: Option<String>, http_client: Arc<dyn HttpClient>) -> Result<Arc<Self>> {
        let server_url = server_url.as_deref().unwrap_or(etebase::DEFAULT_SERVER_URL);
        let inner =
            etebase::Client::new_with_impl(server_url, Box::new(ForeignHttpClient(http_client)))?;
        Ok(Arc::new(Self {
            inner: Mutex::new(inner),
        }))
    }

    /// Points this client at another server. Existing accounts keep their own copy.
    pub fn set_server_url(&self, server_url: String) -> Result<()> {
        Ok(lock(&self.inner).set_server_url(&server_url)?)
    }
}

/// Whether `client` points to an Etebase server.
#[uniffi::export]
pub fn is_etebase_server(client: &Client) -> Result<bool> {
    Ok(client.snapshot().is_server_valid()?)
}
