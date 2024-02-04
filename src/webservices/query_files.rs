use std::path::PathBuf;

use actix_multipart::form::text::Text;
use actix_web::{web, Either, HttpResponse, Responder};
use handlebars::Handlebars;
use serde_json::json;

use super::utilities::multitype_input::{EitherInputExtended, EitherInputExtendedWrapper};

#[derive(serde::Deserialize)]
pub(super) struct QueryFilterRequest {
    query: String,
}

#[derive(actix_multipart::form::MultipartForm)]
pub(super) struct QueryFilterRequestMultipart {
    query: Text<String>,
}

pub(super) async fn handle(
    request: EitherInputExtended<QueryFilterRequest,QueryFilterRequestMultipart>,
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
) -> impl Responder + '_ {
    let request_wrapper = EitherInputExtendedWrapper(request);
    let request = (&request_wrapper).into();
    let query: &str = match request {
        Either::Left(query) => query.query.as_str(),
        Either::Right(query) => query.query.as_str(),
    };
    
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
