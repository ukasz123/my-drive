use std::path::PathBuf;

use actix_web::{web, Either, HttpResponse, Responder};
use handlebars::Handlebars;
use serde_json::json;

#[derive(serde::Deserialize)]
pub (super) struct QueryFilterRequest {
    query: String,
}

pub(super) async fn handle(
    request: web::Form<QueryFilterRequest>,
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
) -> impl Responder + '_ {
    let query = &request.query;

    let files = crate::drive_access::query_files(query, base_dir.as_ref());
    match files {
        Ok(files) => {
            let response = super::response_renderer::ResponseRenderer::new(
                json!({ "files": files }),
                "query_results",
                hb.into_inner().clone(),
            );
            Either::Left(response)
        }
        Err(e) => Either::Right(
            HttpResponse::InternalServerError().body(format!("Error querying files: {}", e)),
        ),
    }
}
