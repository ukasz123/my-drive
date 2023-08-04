use std::{fs, path::PathBuf};

use actix_web::{dev::Service, web, App, Either, HttpRequest, HttpResponse, HttpServer, Responder, middleware::DefaultHeaders};
use anyhow::Result;
use drive_access::FilesResult;
use glob::glob;
use handlebars::Handlebars;
use serde_json::json;
use tracing::debug;

mod drive_access;
mod handlebars_utils;
mod server;
mod validators;

async fn index(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    path: web::ReqData<server::RequestedPath>,
) -> impl Responder {
    let path = path.into_inner().into_inner();
    let data = handle_list_files_request(&path, &base_dir).await;
    debug!("files_listing: {:?}", data);
    match data {
        Ok(data) => match data {
            Either::Left(data) => {
                let body = hb.render("index", &data).unwrap();
                HttpResponse::Ok().body(body)
            }
            Either::Right(resp) => resp,
        },
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn folder_contents(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    path: web::ReqData<server::RequestedPath>,
) -> impl Responder {
    let path = path.into_inner().into_inner();
    let data = handle_list_files_request(&path, &base_dir).await;
    match data {
        Ok(data) => match data {
            Either::Left(data) => {
                let body = hb.render("files_listing", &data).unwrap();
                HttpResponse::Ok().body(body)
            }
            Either::Right(resp) => resp,
        },
        Err(anyhow_err) => match anyhow_err.downcast_ref::<FileListInputError>() {
            Some(err) => HttpResponse::BadRequest().body(err.to_string()),
            None => HttpResponse::InternalServerError().finish(),
        },
    }
}

#[derive(Debug, thiserror::Error)]
enum FileListInputError {
    #[error("Invalid path: {0:?}")]
    InvalidPath(PathBuf),
}

async fn handle_list_files_request(
    path: &PathBuf,
    base_dir: &PathBuf,
) -> Result<Either<FilesResult, HttpResponse>> {
    if path.is_file() {
        let file_type = FileType::try_from(path.as_path())?;
        return Ok(Either::Right(
            HttpResponse::Ok()
                .content_type(file_type.mime)
                .body(fs::read(path)?),
        ));
    }
    let data = drive_access::list_files(&path, &base_dir).await?;
    debug!("files_listing: {:?}", data);
    Ok(Either::Left(data))
}

#[derive(serde::Deserialize)]
struct QueryFilterRequest {
    query: String,
}

#[actix_web::post("/")]
async fn query_files(
    request: web::Form<QueryFilterRequest>,
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
) -> impl Responder {
    let query = &request.query;

    let result = glob(&format!(
        "{}/**/{}*",
        &base_dir.as_os_str().to_str().unwrap(),
        query
    ));
    match result {
        Ok(paths) => {
            let files = paths
                .filter_map(|p| p.ok())
                .map(|path| {
                    let is_dir = path.is_dir();
                    FileInfo {
                        name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                        is_dir: is_dir,
                        file_type: if is_dir {
                            None
                        } else {
                            Some((path.as_path()).try_into().unwrap_or_default())
                        },
                    }
                })
                .filter(|f| !f.name.starts_with(".")) // ignore hidden files
                .collect::<Vec<_>>();
            let body = hb
                .render("query_results", &json!({ "files": files }))
                .unwrap();
            HttpResponse::Ok().body(body)
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

use actix_multipart::Multipart;

use crate::drive_access::{FileType, FileInfo};

#[derive(Debug, actix_multipart::form::MultipartForm)]
struct UploadForm {
    #[multipart(rename = "file")]
    files: Vec<actix_multipart::form::tempfile::TempFile>,
}

async fn upload_file(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    form: actix_multipart::form::MultipartForm<UploadForm>,
    req: HttpRequest,
) -> impl Responder {
    let path: PathBuf = req.match_info().query("path").parse().unwrap();
    let dir_path = base_dir.join(path).to_path_buf();
    let files = form.into_inner().files;
    let results = files
        .into_iter()
        .filter(|file| file.file_name.is_some())
        .map(|file| {
            let name = file.file_name.unwrap();
            let path = dir_path.join(&name);
            (name, file.file.persist(path))
        });
    let summary = results
        .map(|(name, r)| match r {
            Ok(_) => {
                format!("{} file saved", name)
            }
            Err(e) => {
                format!("{} file failed to save: {}", name, e)
            }
        })
        .collect::<String>();
    let data = drive_access::list_files(&dir_path, &base_dir).await;
    match data {
        Ok(data) => {
            let body = hb.render("files_listing", &data).unwrap();
            HttpResponse::Ok().body(format!("{}/n{}", body, summary))
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    // Handlebars uses a repository for the compiled templates. This object must be
    // shared between the application threads, and is therefore passed to the
    // Application Builder as an atomic reference-counted pointer.

    let handlebars = handlebars_utils::prepare();
    let handlebars_ref = web::Data::new(handlebars);

    let base_dir = PathBuf::from(dotenv::var("BASE_DIR").unwrap());
    let base_dir_data = web::Data::new(base_dir);
    HttpServer::new(move || {
        App::new()
            .wrap(tracing_actix_web::TracingLogger::default())
            .service(actix_files::Files::new("/static", "./static"))
            .app_data(base_dir_data.clone())
            .app_data(handlebars_ref.clone())
            .service(query_files)
            .service(
                web::resource("/{path:.*}")
                    .wrap(crate::server::RequestPath)
                    .app_data(
                        actix_multipart::form::MultipartFormConfig::default()
                            .total_limit(1024 * 1024 * 128),
                    )
                    .route(
                        web::get()
                            .guard(actix_web::guard::Header("HX-Request", "true"))
                            .to(folder_contents),
                    )
                    .route(web::get().to(index))
                    .route(web::put().to(upload_file)),
            )
    })
    .bind((
        "127.0.0.1",
        dotenv::var("PORT")
            .unwrap_or("8080".to_owned())
            .parse::<u16>()
            .unwrap(),
    ))?
    .run()
    .await
}
