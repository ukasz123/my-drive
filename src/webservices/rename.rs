use std::path::PathBuf;

use actix_web::{web, Either, HttpResponse, Responder};
use handlebars::Handlebars;
use serde_json::json;
use tokio::fs;
use tokio_stream::StreamExt;
use tracing::{debug, event, info_span, Instrument as _, Level};

use crate::{
    drive_access::{self, FileInfo},
    webservices::response_renderer::ResponseRenderer,
};

pub(super) async fn handle(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    mut form: actix_multipart::Multipart,
    path: web::ReqData<crate::server::RequestedPath>,
) -> impl Responder + '_ {
    let instrumented_fut = async {
        let path = path.as_ref();
        let file_path = base_dir.join(path).to_path_buf();
        if file_path.exists() {
            let file_name = file_path.file_name().map(|os_str| os_str.to_str().unwrap());
            let file_name = match file_name {
                Some(n) => n,
                None => {
                    return Either::Right(HttpResponse::InternalServerError().body("Cannot find file"));
                },
            };
            let field_name = format!("name{}", file_name);
            let mut new_name_field = None;
            while let Some(field) = form.next().await {
                if let Ok(field_val) = field {
                    if field_val.name() == field_name {
                        new_name_field = Some(field_val);
                        break;
                    }
                }
            }

            let mut new_name_field = match new_name_field {
                Some(f) => f,
                None => {
                    return Either::Right(HttpResponse::BadRequest().body("No name field found"));
                }
            };
            let mut new_name = String::new();
            while let Some(chunk) = new_name_field.next().await {
                let chunk = match chunk {
                    Ok(c) => {c},
                    Err(e) => {
                        return Either::Right(HttpResponse::BadRequest().body(format!("Could not read name: {:?}",e)));
                    },
                };

                new_name.push_str(&str::from_utf8(&chunk).unwrap());
            };
            let renamed = file_path
                .parent()
                .map(|parent| parent.join(new_name))
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
