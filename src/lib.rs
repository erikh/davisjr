/// Application/Server-level management and routing configuration and testing support; outermost functionality.
pub mod app;
/// Error types that davisjr uses
pub mod errors;
/// Handler construction and prototypes
pub mod handler;
/// Macros for quality-of-life when interacting with Handlers
pub mod macros;
/// Path management for Routes
pub(crate) mod path;
/// Router, Route management and organization
pub(crate) mod router;

use http::{Request, Response};
use std::{collections::BTreeMap, pin::Pin};

/// Params are a mapping of name -> parameter for the purposes of routing.
pub type Params = BTreeMap<String, String>;

pub(crate) type PinBox<F> = Pin<Box<F>>;

/// HTTPResult is the return type for handlers. If a handler terminates at the end of its chain
/// with [std::option::Option::None] as the [http::Response], a 500 Internal Server Error will be
/// returned. If you wish to return Err(), a [http::StatusCode] or [std::string::String] can be
/// returned, the former is resolved to its status with an empty body, and the latter corresponds
/// to a 500 Internal Server Error with the body set to the string.
pub type HTTPResult<TransientState> = Result<
    (
        Request<hyper::Body>,
        Option<Response<hyper::Body>>,
        TransientState,
    ),
    crate::errors::Error,
>;

/// TransientState must be implemented to use state between handlers.
pub trait TransientState
where
    Self: Clone + Send,
{
    /// initial prescribes an initial state for the trait, allowing it to be constructed at
    /// dispatch time.
    fn initial() -> Self;
}

/// NoState is an empty [crate::TransientState].
#[derive(Clone)]
pub struct NoState;

impl TransientState for NoState {
    fn initial() -> Self {
        Self {}
    }
}

/// A convenience import to gather all of `davisjr`'s dependencies in one easy place.
/// To use:
///
/// ```
///     use davisjr::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        app::App, compose_handler, errors::*, HTTPResult, NoState, Params, TransientState,
    };
    pub use http::{Request, Response, StatusCode};
    pub use hyper::Body;
}
