use actix_web::{web, HttpResponse, Responder};
use handlebars::Handlebars;
use serde_json::json;
use tracing::trace_span;

use std::path::PathBuf;

pub(super) async fn handle(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    path: web::ReqData<crate::server::RequestedPath>,
) -> impl Responder {
    let path = path.as_ref();
    let dir_path = base_dir.join(path).to_path_buf();

    let data = {
        let span = trace_span!("delete file or directory", path = path.to_str());
        let _enter = span.enter();
        crate::drive_access::delete_file_or_directory(&dir_path)
    };
    match data {
        Ok(_) => {
            let span = trace_span!(
                "list files after deletion",
                path = dir_path.parent().and_then(|parent| parent.to_str())
            );
            let _enter = span.enter();
            let data = crate::drive_access::list_files(
                &dir_path.parent().unwrap().to_path_buf(),
                &base_dir,
            )
            .await;
            match data {
                Ok(data) => {
                    let body = hb.render("files_listing", &data).unwrap();
                    let confirmation_toast = hb
                        .render("confirmation_toast", &json!({ "message": "File deleted" }))
                        .unwrap();
                    HttpResponse::Ok().body(format!("{}{}", body, confirmation_toast))
                }
                Err(_) => HttpResponse::InternalServerError()
                    .reason("Failed to fetch files")
                    .finish(),
            }
        }
        Err(_) => HttpResponse::InternalServerError()
            .reason("Failed to delete file")
            .finish(),
    }
}
