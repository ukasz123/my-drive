use std::path::PathBuf;

use actix_multipart::form::text::Text;
use actix_web::{web, Either, HttpResponse, Responder};
use handlebars::Handlebars;
use serde_json::json;
use tokio::fs;
use tracing::{debug, event, info_span, Instrument as _, Level};

use crate::{
    drive_access::{self, FileInfo},
    webservices::response_renderer::ResponseRenderer,
};

#[derive(Debug, actix_multipart::form::MultipartForm)]
pub(super) struct RenameForm {
    name: Text<String>,
}

pub(super) async fn handle(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    form: actix_multipart::form::MultipartForm<RenameForm>,
    path: web::ReqData<crate::server::RequestedPath>,
) -> impl Responder + '_ {
    let instrumented_fut = async {
        let path = path.as_ref();
        let file_path = base_dir.join(path).to_path_buf();
        if file_path.exists() {
            let renamed = file_path
                .parent()
                .map(|parent| parent.join(form.name.0.clone()))
                .ok_or_else(|| Either::Right(HttpResponse::InternalServerError().body("Cannot find file")));
            match renamed {
                Ok(new_path) => {
                    let rename = fs::rename(file_path, &new_path).await;
                    match rename {
                        Ok(_) => {
                            let fi = FileInfo::from(new_path.clone());
                            debug!(file = ?fi, "Updated file");
                            let data = json!({
                                "file": fi,
                                "path": drive_access::relative_path(&new_path.parent().unwrap(), &base_dir).unwrap()
                            });
                            Either::Left(ResponseRenderer::new(data, "files_row", hb.into_inner().clone()))

                        }
                        Err(_) => Either::Right(HttpResponse::InternalServerError().body("Cannot rename file")),
                    }
                }
                Err(err_response) => err_response,
            }
        } else {
            event!(
                Level::INFO,
                path = path.as_os_str().to_str(),
                "File not found"
            );
            Either::Right(HttpResponse::NotFound().finish())
        }
    }
    .instrument(info_span!("handle_rename"));

    instrumented_fut.await
}
