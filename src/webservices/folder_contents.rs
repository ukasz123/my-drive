use actix_web::{http::header, web, Either, HttpResponse, Responder};
use handlebars::Handlebars;
use std::path::PathBuf;
use tracing::{debug_span, event, info_span, Instrument, Level};

use super::list_files::list_files_or_file_contents;

pub(super) async fn handle(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    path: web::ReqData<crate::server::RequestedPath>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let span = info_span!("handle_path");
    let instrumented_span = async {
        let path = path.into_inner().into();
        let data = list_files_or_file_contents(&path, &base_dir).await;
        match data {
            Ok(data) => match data {
                Either::Left(data) => {
                    let body = debug_span!("render_files_listing")
                        .in_scope(|| hb.render("files_listing", &data))
                        .unwrap();
                    HttpResponse::Ok()
                        .insert_header(header::ContentType::html())
                        .body(body)
                }
                Either::Right(file) => {
                    debug_span!("return_file").in_scope(|| file.into_response(&req))
                }
            },
            Err(anyhow_err) => {
                event!(Level::WARN, "error: {:?}", anyhow_err);
                match anyhow_err.downcast_ref::<super::FileListInputError>() {
                    Some(err) => HttpResponse::BadRequest().body(err.to_string()),
                    None => HttpResponse::NotFound().finish(),
                }
            }
        }
    }
    .instrument(span);
    instrumented_span.await
}
