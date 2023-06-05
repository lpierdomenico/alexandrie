use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use tide::{Request, StatusCode};

use crate::db::models::Crate;
use crate::db::schema::*;
use crate::error::AlexError;
use crate::utils;
use crate::State;

/// Response body for this route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseBody {
    /// The crate's name.
    pub name: String,
    /// The crate's description.
    pub description: Option<String>,
    /// The crate's repository link.
    pub repository: Option<String>,
    /// The crate's documentation link.
    pub documentation: Option<String>,
    /// The crate's download count.
    pub downloads: i64,
    /// The crate's creation date.
    pub created_at: String,
    /// The crate's last modification date.
    pub updated_at: String,
    /// The crate's keywords.
    pub keywords: Vec<String>,
    /// The crate's categories.
    pub categories: Vec<String>,
}

/// Route to get information about a crate.
pub async fn get(req: Request<State>) -> tide::Result {
    let name = req.param("name")?.to_string();

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


    //? Fetch the crate data from the database.
    let krate = db
        .run(move |conn| {
            crates::table
                .filter(crates::canon_name.eq(name.as_str()))
                .first::<Crate>(conn)
                .optional()
        })
        .await?;

    //? Was a crate found ?
    let krate = match krate {
        Some(krate) => krate,
        None => {
            return Ok(utils::response::error(
                StatusCode::NotFound,
                "the crate could not be found",
            ))
        }
    };

    //? Fetch the crate's keywords
    let crate_id = krate.id;
    let keywords = db
        .run(move |conn| {
            crate_keywords::table
                .inner_join(keywords::table)
                .select(keywords::name)
                .filter(crate_keywords::crate_id.eq(crate_id))
                .load::<String>(conn)
        })
        .await?;

    //? Fetch the crate's categories
    let crate_id = krate.id;
    let categories = db
        .run(move |conn| {
            crate_categories::table
                .inner_join(categories::table)
                .select(categories::tag)
                .filter(crate_categories::crate_id.eq(crate_id))
                .load::<String>(conn)
        })
        .await?;

    let response = ResponseBody {
        keywords,
        categories,
        name: krate.name,
        description: krate.description,
        repository: krate.repository,
        documentation: krate.documentation,
        downloads: krate.downloads,
        created_at: krate.created_at,
        updated_at: krate.updated_at,
    };
    Ok(utils::response::json(&response))
}
