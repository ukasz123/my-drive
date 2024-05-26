use actix_multipart::form::text::Text;
use actix_web::{web, Either, HttpResponse, Responder};
use handlebars::Handlebars;
use tracing::trace_span;

use std::path::PathBuf;

use super::utilities::multitype_input::{EitherInputExtended, EitherInputExtendedWrapper};

#[derive(Debug, actix_multipart::form::MultipartForm)]
pub(super) struct NewDirForm {
    #[multipart(rename = "new_folder")]
    new_folder_name: Text<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct NewDirRequest {
    new_folder_name: String,
}

pub(super) async fn handle(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    form: EitherInputExtended<NewDirRequest, NewDirForm>,
    path: web::ReqData<crate::server::RequestedPath>,
) -> impl Responder {
    let path = path.as_ref();
    let dir_path = base_dir.join(path).to_path_buf();
    // create new folder
    let form_wrapper = EitherInputExtendedWrapper(form);
    let form = (&form_wrapper).into();
    let new_dir_name = match form {
        Either::Left(form) => form.new_folder_name.as_str(),
        Either::Right(form) => form.new_folder_name.as_str(),
    };

    let new_dir_path = dir_path.join(new_dir_name);
    let data = 
    { 
        let span = trace_span!("create dir", path=new_dir_path.to_str());
        let _enter = span.enter();
        crate::drive_access::create_dir(&new_dir_path)
    };
    match data {
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
    }
}
