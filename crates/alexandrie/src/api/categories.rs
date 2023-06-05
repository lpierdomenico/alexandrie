use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use tide::Request;

use crate::db::models::Category;
use crate::db::schema::*;
use crate::error::AlexError;
use crate::utils;
use crate::State;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CategoriesResponse {
    pub categories: Vec<CategoriesResult>,
    pub meta: CategoriesMeta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CategoriesResult {
    pub name: String,
    pub tag: String,
    pub description: String,
}

impl From<Category> for CategoriesResult {
    fn from(category: Category) -> CategoriesResult {
        CategoriesResult {
            name: category.name,
            tag: category.tag,
            description: category.description,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CategoriesMeta {
    pub total: usize,
}

/// Route to list categories.
pub(crate) async fn get(req: Request<State>) -> tide::Result {
    let state = req.state();
    let db = &state.db;

    let headers = req
        .header(utils::auth::AUTHORIZATION_HEADER)
        .ok_or(AlexError::InvalidToken)?;
    let header = headers.last().to_string();
    let author = db
        .run(move |conn| utils::checks::get_author(conn, header))
        .await
        .ok_or(AlexError::InvalidToken)?;

    let categories = db
        .run(|conn| categories::table.load::<Category>(conn))
        .await?;

    let categories: Vec<_> = categories.into_iter().map(CategoriesResult::from).collect();
    let total = categories.len();

    let data = CategoriesResponse {
        categories,
        meta: CategoriesMeta { total },
    };
    Ok(utils::response::json(&data))
}
