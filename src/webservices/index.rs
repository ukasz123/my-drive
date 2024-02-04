use actix_web::{web, Either, HttpResponse, Responder};
use handlebars::Handlebars;
use std::path::PathBuf;

use super::{list_files::list_files_or_file_contents, response_renderer::ResponseRenderer};

pub(crate) async fn handle(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    path: web::ReqData<crate::server::RequestedPath>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let path = path.into_inner().into();
    let data = list_files_or_file_contents(&path, &base_dir).await;
    match data {
        Ok(data) => match data {
            Either::Left(data) => {
                ResponseRenderer::new(data, "index", hb.into_inner().clone()).respond_to(&req).map_into_boxed_body()
            }
            Either::Right(resp) => resp.into_response(&req),
        },
        Err(anyhow_err) => match anyhow_err.downcast_ref::<super::FileListInputError>() {
            Some(err) => HttpResponse::BadRequest().body(err.to_string()),
            None => HttpResponse::NotFound().finish(),
        },
    }
}
