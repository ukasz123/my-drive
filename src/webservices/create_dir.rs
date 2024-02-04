use actix_multipart::form::text::Text;
use actix_web::{web, HttpResponse, Responder};
use handlebars::Handlebars;

use std::path::PathBuf;

#[derive(Debug, actix_multipart::form::MultipartForm)]
pub(super) struct NewDirForm {
    #[multipart(rename = "new_folder")]
    new_folder_name: Text<String>,
}

pub(super) async fn handle(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    form: actix_multipart::form::MultipartForm<NewDirForm>,
    path: web::ReqData<crate::server::RequestedPath>,
) -> impl Responder {
    let path = path.as_ref();
    let dir_path = base_dir.join(path).to_path_buf();
    // create new folder
    let new_dir_name = &form.new_folder_name;

    let new_dir_path = dir_path.join(new_dir_name.as_str());
    let data = crate::drive_access::create_dir(&new_dir_path);
    return match data {
        Ok(_) => {
            let data = crate::drive_access::list_files(&dir_path, &base_dir).await;
            match data {
                Ok(data) => {
                    let body = hb.render("files_listing", &data).unwrap();
                    HttpResponse::Ok().body(body)
                }
                Err(_) => HttpResponse::InternalServerError().finish(),
            }
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    };
}
