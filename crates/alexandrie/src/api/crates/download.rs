use async_std::io;

use diesel::prelude::*;
use semver::Version;
use tide::http::mime;
use tide::{Body, Request, Response, StatusCode};

use alexandrie_storage::Store;

use crate::db::schema::*;
use crate::error::{AlexError, Error};
use crate::utils;
use crate::State;

/// Route to download a crate's tarball (used by `cargo build`).
///
/// The response is streamed, for performance and memory footprint reasons.
pub(crate) async fn get(req: Request<State>) -> tide::Result {
    let name = req.param("name")?.to_string();
    let version: Version = req.param("version")?.parse()?;

    let name = utils::canonical_name(name);

    let state = req.state().clone();
    let db = &state.db;

    let headers = req
        .header(utils::auth::AUTHORIZATION_HEADER)
        .ok_or(AlexError::InvalidToken)?;
    let header = headers.last().to_string();
    let author = db
        .run(move |conn| utils::checks::get_author(conn, header))
        .await
        .ok_or(AlexError::InvalidToken)?;

    // state.index.refresh()?;

    let transaction = db.transaction(move |conn| {
        let state = req.state();

        //? Fetch the download count for this crate.
        let crate_info = crates::table
            .select((crates::name, crates::downloads))
            .filter(crates::canon_name.eq(name.as_str()))
            .first::<(String, i64)>(conn)
            .optional()?;

        if let Some((name, downloads)) = crate_info {
            //? Increment this crate's download count.
            diesel::update(crates::table.filter(crates::name.eq(name.as_str())))
                .set(crates::downloads.eq(downloads + 1))
                .execute(conn)?;
            let mut krate = state.storage.read_crate(&name, version)?;
            let mut buf = Vec::new();
            krate.read_to_end(&mut buf)?;
            let mut response = Response::new(StatusCode::Ok);
            response.set_content_type(mime::BYTE_STREAM);
            response.set_body(Body::from_reader(io::Cursor::new(buf), None));
            Ok(response)
        } else {
            Err(Error::from(AlexError::CrateNotFound { name }))
        }
    });

    Ok(transaction.await?)
}
