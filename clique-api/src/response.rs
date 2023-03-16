//! Various types to encode responses from the API.
use rocket::{
    http::Status,
    response::Responder,
    serde::{json::Json, msgpack::MsgPack},
    Request,
};
use serde::Serialize;

pub type Response<T> = Result<Object<T>, Error>;

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl From<clique_db::Error> for ErrorBody {
    fn from(err: clique_db::Error) -> Self {
        Self {
            code: "database_connection",
            message: err.to_string(),
        }
    }
}

pub struct Error {
    error: Object<ErrorBody>,
    status: Status,
}

impl Error {
    pub const fn new(code: &'static str, message: String, status: Status) -> Self {
        Self {
            error: Object(ErrorBody { code, message }),
            status,
        }
    }
}

impl From<clique_db::Error> for Error {
    fn from(err: clique_db::Error) -> Self {
        Self {
            error: Object(err.into()),
            status: Status::InternalServerError,
        }
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'o> {
        (self.status, self.error).respond_to(req)
    }
}

pub struct Object<T: Serialize>(pub T);

impl<'r, 'o: 'r, T: Serialize> Responder<'r, 'o> for Object<T> {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'o> {
        let accepts_msgpack = req
            .accept()
            .map(|accept| {
                accept
                    .media_types()
                    .any(|mt| mt.top() == "application" && mt.sub() == "msgpack")
            })
            .unwrap_or_default();
        if accepts_msgpack {
            MsgPack(self.0).respond_to(req)
        } else {
            Json(self.0).respond_to(req)
        }
    }
}
