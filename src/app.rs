use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use http::{HeaderMap, Method, Request, Response, StatusCode};
use hyper::{server::conn::Http, service::service_fn, Body};
use tokio::{net::TcpListener, sync::Mutex};

#[cfg(feature = "unix")]
use std::path::PathBuf;
#[cfg(feature = "unix")]
use tokio::net::UnixListener;

use crate::{errors::*, handler::Handler, router::Router, TransientState};

/// App is used to define application-level functionality and initialize the server. Routes are
/// typically programmed here.
///
/// ```ignore
///   async fn item(
///         req: Request<Body>,
///         resp: Option<Response<Body>>,
///         params: Params,
///         app: App<NoState>
///         state: NoState,
///   ) -> HTTPResult<NoState> {
///     println!("item: {}", params["item"]);
///     println!("route: {}", params["*"]);
///
///     Ok((
///        req,
///        Response::builder().
///             status(StatusCode::OK).
///             body(Body::default()).
///             unwrap(),
///        NoState {},
///     ))
///   }
///
///   #[tokio::main]
///   async fn main() -> Result<(), ServerError> {
///     let app = App::new();
///     app.get("/*/item/:item", compose_handler!(item))?;
///     app.serve("localhost:0").await
///   }
/// ```
///
/// Note that App here has _no state_. It will have a type signature of `App<NoState>`. To carry state,
/// look at the `with_state` method which will change the type signature of the `item` call (and
/// other handlers).
///
/// App routes take a Path: a Path is a URI path component that has the capability to superimpose
/// variables. Paths are really simple but useful for capturing dynamic parts of a routing path.
///
/// Paths should always start with `/`. Paths that are dynamic have member components that start
/// with `:`. For example, `/a/b/c` will always only match one route, while `/a/:b/c` will match
/// any route with `/a/<any single path component>/c`.
///
/// A single wildcard can be specified with `*`. Its parameter will be called '*' and will
/// correspond to the inner path that filled it. Currently, multiple paths that match due to a
/// wildcard are undefined behavior; and there are some rules around wildcard paths:
///
/// - There may be only one wildcard component in a path (for now; this may change)
/// - Wildcard components may not be immediately followed by a named parameter, due to ambiguity.
///
/// Variadic path components are accessible through the [crate::Params] implementation. Paths are
/// typically used through [crate::app::App] methods that use a string form of the Path.
///
/// Requests are routed through paths to [crate::handler::HandlerFunc]s.
#[derive(Clone)]
pub struct App<S: Clone + Send, T: TransientState + 'static + Clone + Send> {
    router: Router<S, T>,
    global_state: Option<Arc<Mutex<S>>>,
}

impl<S: 'static + Clone + Send, T: TransientState + 'static + Clone + Send> App<S, T> {
    /// Construct a new App with no state; it will be passed to handlers as `App<()>`.
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            global_state: None,
        }
    }

    /// Construct an App with state.
    ///
    /// This has the type `App<S>` where S is `+ 'static + Clone + Send` and will be passed to
    /// handlers with the appropriate concrete type.
    ///
    pub fn with_state(state: S) -> Self {
        Self {
            router: Router::new(),
            global_state: Some(Arc::new(Mutex::new(state))),
        }
    }

    // FIXME Currently you must await this, seems pointless.
    /// Return the state of the App. This is returned as `Arc<Mutex<S>>` and must be acquired under
    /// lock. In situations where there is no state, [std::option::Option::None] is returned.
    pub async fn state(&self) -> Option<Arc<Mutex<S>>> {
        self.global_state.clone()
    }

    /// Create a route for a GET request. See App's docs and [crate::handler::Handler] for
    /// more information.
    pub fn get(&mut self, path: &str, ch: Handler<S, T>) -> Result<(), ServerError> {
        self.router.add(Method::GET, path.to_string(), ch)?;
        Ok(())
    }

    /// Create a route for a POST request. See App's docs and [crate::handler::Handler] for
    /// more information.
    pub fn post(&mut self, path: &str, ch: Handler<S, T>) -> Result<(), ServerError> {
        self.router.add(Method::POST, path.to_string(), ch)?;
        Ok(())
    }

    /// Create a route for a DELETE request. See App's docs and [crate::handler::Handler] for
    /// more information.
    pub fn delete(&mut self, path: &str, ch: Handler<S, T>) -> Result<(), ServerError> {
        self.router.add(Method::DELETE, path.to_string(), ch)?;
        Ok(())
    }

    /// Create a route for a PUT request. See App's docs and [crate::handler::Handler] for
    /// more information.
    pub fn put(&mut self, path: &str, ch: Handler<S, T>) -> Result<(), ServerError> {
        self.router.add(Method::PUT, path.to_string(), ch)?;
        Ok(())
    }

    /// Create a route for an OPTIONS request. See App's docs and
    /// [crate::handler::Handler] for more information.
    pub fn options(&mut self, path: &str, ch: Handler<S, T>) -> Result<(), ServerError> {
        self.router.add(Method::OPTIONS, path.to_string(), ch)?;
        Ok(())
    }

    /// Create a route for a PATCH request. See App's docs and
    /// [crate::handler::Handler] for more information.
    pub fn patch(&mut self, path: &str, ch: Handler<S, T>) -> Result<(), ServerError> {
        self.router.add(Method::PATCH, path.to_string(), ch)?;
        Ok(())
    }

    /// Create a route for a HEAD request. See App's docs and
    /// [crate::handler::Handler] for more information.
    pub fn head(&mut self, path: &str, ch: Handler<S, T>) -> Result<(), ServerError> {
        self.router.add(Method::HEAD, path.to_string(), ch)?;
        Ok(())
    }

    /// Create a route for a CONNECT request. See App's docs and
    /// [crate::handler::Handler] for more information.
    pub fn connect(&mut self, path: &str, ch: Handler<S, T>) -> Result<(), ServerError> {
        self.router.add(Method::CONNECT, path.to_string(), ch)?;
        Ok(())
    }

    /// Create a route for a TRACE request. See App's docs and
    /// [crate::handler::Handler] for more information.
    pub fn trace(&mut self, path: &str, ch: Handler<S, T>) -> Result<(), ServerError> {
        self.router.add(Method::TRACE, path.to_string(), ch)?;
        Ok(())
    }

    /// Dispatch a route based on the request. Returns a response based on the error status of the
    /// handler chain following the normal chain of responsibility rules described elsewhere. Only
    /// needed by server implementors.
    pub async fn dispatch(&self, req: Request<Body>) -> Result<Response<Body>, Infallible> {
        let _uri = req.uri().clone();
        let _method = req.method().clone();

        #[cfg(all(feature = "logging", not(feature = "trace")))]
        log::info!("{} request to {}", _method, _uri);

        #[cfg(feature = "trace")]
        tracing::info!("{} request to {}", _method, _uri);

        match self.router.dispatch(req, self.clone()).await {
            Ok(resp) => {
                let _status = resp.status().clone();

                #[cfg(all(feature = "logging", not(feature = "trace")))]
                log::info!(
                    "{} request to {}: responding with status {}",
                    _method,
                    _uri,
                    _status,
                );

                #[cfg(feature = "trace")]
                tracing::info!(
                    "{} request to {}: responding with status {}",
                    _method,
                    _uri,
                    _status,
                );

                Ok(resp)
            }
            Err(e) => {
                #[cfg(all(feature = "logging", not(feature = "trace")))]
                log::error!(
                    "{} request to {}: responding with error {:?}",
                    _method,
                    _uri,
                    e,
                );

                #[cfg(feature = "trace")]
                tracing::error!(
                    "{} request to {}: responding with error {:?}",
                    _method,
                    _uri,
                    e,
                );
                match e.clone() {
                    Error::StatusCode(sc, msg) => Ok(Response::builder()
                        .status(sc)
                        .body(Body::from(msg))
                        .unwrap()),
                    Error::InternalServerError(e) => Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from(e.to_string()))
                        .unwrap()),
                }
            }
        }
    }

    #[cfg(feature = "unix")]
    pub async fn serve_unix(self, filename: PathBuf) -> Result<(), ServerError> {
        let unix_listener = UnixListener::bind(filename)?;
        loop {
            let (stream, _) = unix_listener.accept().await?;

            let s = self.clone();
            let sfn = service_fn(move |req: Request<Body>| {
                let s = s.clone();
                async move { s.clone().dispatch(req).await }
            });

            tokio::task::spawn(async move {
                if let Err(http_err) = Http::new()
                    .http1_keep_alive(true)
                    .serve_connection(stream, sfn)
                    .await
                {
                    #[cfg(feature = "logging")]
                    log::error!("ServerError while serving HTTP connection: {}", http_err);
                    #[cfg(feature = "trace")]
                    tracing::error!("ServerError while serving HTTP connection: {}", http_err);
                    #[cfg(all(not(feature = "trace"), not(feature = "logging")))]
                    eprintln!("ServerError while serving HTTP connection: {}", http_err);
                }
            });
        }
    }

    /// Start a TCP/HTTP server with tokio. Performs dispatch on an as-needed basis. This is a more
    /// common path for users to start a server.
    pub async fn serve(self, addr: &str) -> Result<(), ServerError> {
        let socketaddr: SocketAddr = addr.parse()?;

        let tcp_listener = TcpListener::bind(socketaddr).await?;
        loop {
            let (tcp_stream, sa) = tcp_listener.accept().await?;

            let s = self.clone();
            let sfn = service_fn(move |mut req: Request<Body>| {
                let ip = sa.ip();
                req.extensions_mut().insert(ip);
                let s = s.clone();
                async move { s.clone().dispatch(req).await }
            });

            #[cfg(all(feature = "logging", not(feature = "trace")))]
            log::trace!("Request from {}", sa);

            #[cfg(feature = "trace")]
            tracing::trace!("Request from {}", sa);

            tokio::task::spawn(async move {
                if let Err(http_err) = Http::new()
                    .http1_keep_alive(true)
                    .serve_connection(tcp_stream, sfn)
                    .await
                {
                    #[cfg(feature = "logging")]
                    log::error!("ServerError while serving HTTP connection: {}", http_err);
                    #[cfg(feature = "trace")]
                    tracing::error!("ServerError while serving HTTP connection: {}", http_err);
                    #[cfg(all(not(feature = "trace"), not(feature = "logging")))]
                    eprintln!("ServerError while serving HTTP connection: {}", http_err);
                }
            });
        }
    }

    /// Start a TLS-backed TCP/HTTP server with tokio. Performs dispatch on an as-needed basis. This is a more
    /// common path for users to start a server.
    #[cfg(feature = "tls")]
    pub async fn serve_tls(
        self,
        addr: &str,
        config: tokio_rustls::rustls::ServerConfig,
    ) -> Result<(), ServerError> {
        let socketaddr: SocketAddr = addr.parse()?;

        let config = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let tcp_listener = TcpListener::bind(socketaddr).await?;
        loop {
            let (tcp_stream, sa) = tcp_listener.accept().await?;

            let s = self.clone();
            let sfn = service_fn(move |mut req: Request<Body>| {
                let ip = sa.ip();
                req.extensions_mut().insert(ip);
                let s = s.clone();
                async move { s.clone().dispatch(req).await }
            });

            #[cfg(all(feature = "logging", not(feature = "trace")))]
            log::trace!("Request from {}", sa);

            #[cfg(feature = "trace")]
            tracing::trace!("Request from {}", sa);

            let config = config.clone();
            tokio::task::spawn(async move {
                match config.accept(tcp_stream).await {
                    Ok(tcp_stream) => {
                        if let Err(http_err) = Http::new()
                            .http1_keep_alive(true)
                            .serve_connection(tcp_stream, sfn)
                            .await
                        {
                            #[cfg(feature = "logging")]
                            log::error!("ServerError while serving HTTP connection: {}", http_err);
                            #[cfg(feature = "trace")]
                            tracing::error!(
                                "ServerError while serving HTTP connection: {}",
                                http_err
                            );
                            #[cfg(all(not(feature = "trace"), not(feature = "logging")))]
                            eprintln!("ServerError while serving HTTP connection: {}", http_err);
                        }
                    }
                    Err(e) => {
                        #[cfg(feature = "logging")]
                        log::error!("ServerError while serving TLS: {:?}", e);
                        #[cfg(feature = "trace")]
                        tracing::error!("ServerError while serving TLS: {:?}", e);
                        #[cfg(all(not(feature = "trace"), not(feature = "logging")))]
                        eprintln!("ServerError while serving TLS: {:?}", e);
                    }
                }
            });
        }
    }
}

/// TestApp is a testing framework for davisjr applications. Given an App, it can issue mock
/// requests to it without standing up a typical web server.
#[derive(Clone)]
pub struct TestApp<S: Clone + Send + 'static, T: TransientState + 'static + Clone + Send> {
    app: App<S, T>,
    headers: Option<HeaderMap>,
}

impl<S: Clone + Send + 'static, T: TransientState + 'static + Clone + Send> TestApp<S, T> {
    /// Construct a new tested application.
    pub fn new(app: App<S, T>) -> Self {
        Self { app, headers: None }
    }

    /// with_headers applies the headers to any following request, and acts as an alternative
    /// constructor.
    pub fn with_headers(&self, headers: http::HeaderMap) -> Self {
        Self {
            app: self.app.clone(),
            headers: Some(headers),
        }
    }

    /// dispatch a request to the application, this allows for maximum flexibility.
    pub async fn dispatch(&self, req: Request<Body>) -> Response<Body> {
        self.app.dispatch(req).await.unwrap()
    }

    fn populate_headers(&self, mut req: http::request::Builder) -> http::request::Builder {
        if let Some(include_headers) = self.headers.clone() {
            for (header, value) in include_headers.clone() {
                if let Some(header) = header {
                    req = req.header(header, value.clone());
                }
            }
        }

        req
    }

    /// Perform a GET request against the path.
    pub async fn get(&self, path: &str) -> Response<Body> {
        let req = self.populate_headers(Request::builder());

        self.app
            .dispatch(req.uri(path).body(Body::default()).unwrap())
            .await
            .unwrap()
    }

    /// Perform a POST request against the path.
    pub async fn post(&self, path: &str, body: Body) -> Response<Body> {
        let req = self.populate_headers(Request::builder());

        self.app
            .dispatch(req.method(Method::POST).uri(path).body(body).unwrap())
            .await
            .unwrap()
    }

    /// Perform a DELETE request against the path.
    pub async fn delete(&self, path: &str) -> Response<Body> {
        let req = self.populate_headers(Request::builder());
        self.app
            .dispatch(
                req.method(Method::DELETE)
                    .uri(path)
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// Perform a PUT request against the path.
    pub async fn put(&self, path: &str, body: Body) -> Response<Body> {
        let req = self.populate_headers(Request::builder());
        self.app
            .dispatch(req.method(Method::PUT).uri(path).body(body).unwrap())
            .await
            .unwrap()
    }

    /// Perform an OPTIONS request against the path.
    pub async fn options(&self, path: &str) -> Response<Body> {
        let req = self.populate_headers(Request::builder());
        self.app
            .dispatch(
                req.method(Method::OPTIONS)
                    .uri(path)
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// Perform a PATCH request against the path.
    pub async fn patch(&self, path: &str, body: Body) -> Response<Body> {
        let req = self.populate_headers(Request::builder());
        self.app
            .dispatch(req.method(Method::PATCH).uri(path).body(body).unwrap())
            .await
            .unwrap()
    }

    /// Perform a HEAD request against the path.
    pub async fn head(&self, path: &str) -> Response<Body> {
        let req = self.populate_headers(Request::builder());
        self.app
            .dispatch(
                req.method(Method::HEAD)
                    .uri(path)
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// Perform a TRACE request against the path.
    pub async fn trace(&self, path: &str) -> Response<Body> {
        let req = self.populate_headers(Request::builder());
        self.app
            .dispatch(
                req.method(Method::TRACE)
                    .uri(path)
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// Perform a CONNECT request against the path.
    pub async fn connect(&self, path: &str) -> Response<Body> {
        let req = self.populate_headers(Request::builder());
        self.app
            .dispatch(
                req.method(Method::CONNECT)
                    .uri(path)
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap()
    }
}
