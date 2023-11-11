use anyhow::anyhow;

/// An error for server-related issues.
#[derive(Debug, Clone)]
pub struct ServerError(pub String);

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<std::net::AddrParseError> for ServerError {
    fn from(value: std::net::AddrParseError) -> Self {
        Self(value.to_string())
    }
}

impl From<std::io::Error> for ServerError {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for ServerError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::error::Error for ServerError {}

/// General errors for davisjr handlers. Yield either a StatusCode for a literal status, or a
/// String for a 500 Internal Server Error. Other status codes should be yielded through
/// [http::Response] returns.
#[derive(Clone, Debug)]
pub enum Error {
    StatusCode(http::StatusCode, String),
    InternalServerError(String),
}

impl Default for Error {
    fn default() -> Self {
        Self::InternalServerError("internal server error".to_string())
    }
}

impl Error {
    /// Convenience method to pass anything in that accepts a .to_string method.
    pub fn new<T>(message: T) -> Self
    where
        T: ToString,
    {
        Self::InternalServerError(message.to_string())
    }

    /// A convenient way to return status codes with optional informational bodies.
    pub fn new_status<T>(error: http::StatusCode, message: T) -> Self
    where
        T: ToString,
    {
        Self::StatusCode(error, message.to_string())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StatusCode(code, message) => f.write_str(&format!("{}: {}", code, message)),
            Self::InternalServerError(ise) => f.write_str(&format!("Error: {}", ise.to_string())),
        }
    }
}

impl<T> From<T> for Error
where
    T: std::error::Error,
{
    fn from(value: T) -> Self {
        Self::new(value.to_string())
    }
}

impl Into<anyhow::Error> for Error {
    fn into(self) -> anyhow::Error {
        anyhow!(self.to_string())
    }
}

pub trait ToStatus
where
    Self: ToString,
{
    fn to_status(&self) -> Error;
}
