use actix_web::{web, HttpResponse, Responder};
use handlebars::Handlebars;
use serde_json::json;
use tracing::{event, info_span, trace_span, Instrument, Level};

use std::path::PathBuf;

pub(super) async fn handle(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    path: web::ReqData<crate::server::RequestedPath>,
) -> impl Responder {
    let instrumented_fut = async {
        let path = path.as_ref();
        let dir_path = base_dir.join(path).to_path_buf();

        let data = trace_span!("delete file or directory", path = path.to_str())
            .in_scope(|| crate::drive_access::delete_file_or_directory(&dir_path));
        match data {
            Ok(_) => {
                let data = crate::drive_access::list_files(
                    &dir_path.parent().unwrap().to_path_buf(),
                    &base_dir,
                )
                .instrument(trace_span!(
                    "list files after deletion",
                    path = dir_path.parent().and_then(|parent| parent.to_str())
                ))
                .await;
                match data {
                    Ok(data) => {
                        let body = trace_span!("render_files_listing")
                            .in_scope(|| hb.render("files_listing", &data))
                            .unwrap();
                        let confirmation_toast = trace_span!("render_confirmation_toast")
                            .in_scope(|| {
                                hb.render(
                                    "confirmation_toast",
                                    &json!({ "message": "File deleted" }),
                                )
                            })
                            .unwrap();
                        HttpResponse::Ok().body(format!("{}{}", body, confirmation_toast))
                    }
                    Err(list_err) => {
                        event!(Level::WARN, "error listing files: {:?}", list_err);

                        HttpResponse::InternalServerError()
                            .reason("Failed to fetch files")
                            .finish()
                    }
                }
            }
            Err(del_err) => {
                event!(Level::WARN, "error deleting file: {:?}", del_err);
                HttpResponse::InternalServerError()
                    .reason("Failed to delete file")
                    .finish()
            }
        }
    }
    .instrument(info_span!("handle_delete"));
    instrumented_fut.await
}
