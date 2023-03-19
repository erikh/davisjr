use davisjr::prelude::*;

const DEFAULT_AUTHTOKEN: &str = "867-5309";
const AUTHTOKEN_FILENAME: &str = "authtoken.secret";

async fn validate_authtoken(
    req: Request<Body>,
    resp: Option<Response<Body>>,
    _params: Params,
    _app: App<(), NoState>,
    _state: NoState,
) -> HTTPResult<NoState> {
    let token = req.headers().get("X-AuthToken");
    if token.is_none() {
        return Err(Error::StatusCode(StatusCode::UNAUTHORIZED, String::new()));
    }

    let token = token.unwrap();

    let matches = match std::fs::metadata(AUTHTOKEN_FILENAME) {
        Ok(_) => {
            let s = std::fs::read_to_string(AUTHTOKEN_FILENAME)?;
            s.as_str() == token
        }
        Err(_) => DEFAULT_AUTHTOKEN == token,
    };

    if !matches {
        return Err(Error::StatusCode(StatusCode::UNAUTHORIZED, String::new()));
    }

    return Ok((req, resp, NoState {}));
}

async fn hello(
    req: Request<Body>,
    _resp: Option<Response<Body>>,
    params: Params,
    _app: App<(), NoState>,
    _state: NoState,
) -> HTTPResult<NoState> {
    let name = params.get("name").unwrap();
    let bytes = Body::from(format!("hello, {}!\n", name));

    return Ok((
        req,
        Some(Response::builder().status(200).body(bytes).unwrap()),
        NoState {},
    ));
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let mut app = App::new();
    app.get("/auth/:name", compose_handler!(validate_authtoken, hello));
    app.get("/:name", compose_handler!(hello));

    app.serve("127.0.0.1:3000").await?;

    Ok(())
}
