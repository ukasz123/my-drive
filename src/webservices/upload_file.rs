use actix_web::{http::header, web, HttpResponse, Responder};
use handlebars::Handlebars;
use serde_json::json;
use std::path::PathBuf;

#[derive(Debug, actix_multipart::form::MultipartForm)]
pub(super) struct UploadFile {
    #[multipart(rename = "file")]
    files: Vec<actix_multipart::form::tempfile::TempFile>,
}

pub(super) async fn handle(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    form: actix_multipart::form::MultipartForm<UploadFile>,
    path: web::ReqData<crate::server::RequestedPath>,
    accept_header: web::Header<header::Accept>,
) -> impl Responder {
    let path = path.as_ref();
    let dir_path = base_dir.join(path).to_path_buf();

    // save new files
    let files = form.into_inner().files;
    let results = crate::drive_access::save_files(files, &dir_path);
    let summary = results
        .map(|(name, r)| match r {
            Ok(_) => {
                json!({"message": format!("File {} saved", name), "isError": false})
            }
            Err(e) => {
                json!({"message": format!("File {} failed to save: {}", name, e), "isError": true})
            }
        })
        .collect::<Vec<_>>();
    let data = crate::drive_access::list_files(&dir_path, &base_dir).await;
    match data {
        Ok(data) => {
            let body = hb.render("files_listing", &data).unwrap();
            let summary = hb.render("upload_file_summary_message", &summary).unwrap();
            let confirmation_toast = hb
                .render("confirmation_toast", &json!({ "message": summary }))
                .unwrap();
            if accept_header.iter().any(|h| h.item.subtype() == "json") {
                HttpResponse::Ok().json(json!({"files": data, "message": summary}))
            } else {
                HttpResponse::Ok().body(format!("{}{}", body, confirmation_toast))
            }
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
