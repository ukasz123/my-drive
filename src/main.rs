use std::path::PathBuf;

use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use handlebars::{handlebars_helper, Handlebars};
use tracing::debug;

#[derive(Debug, serde::Serialize)]
struct FileInfo {
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, serde::Serialize)]
struct FilesResult {
    pub files: Vec<FileInfo>,
    pub path: String,
    pub parent: Option<String>,
}

// #[actix_web::get("/")]
async fn index(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    req: HttpRequest,
) -> impl Responder {
    let data = handle_list_files_request(&base_dir, &req).await;
    debug!("files_listing: {:?}", data);
    match data {
        Ok(data) => {
            let body = hb.render("index", &data).unwrap();
            HttpResponse::Ok().body(body)
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn folder_contents(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    req: HttpRequest,
) -> impl Responder {
    let data = handle_list_files_request(&base_dir, &req).await;
    match data {
        Ok(data) => {
            let body = hb.render("files_listing", &data).unwrap();
            HttpResponse::Ok().body(body)
        }
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
    base_dir: &web::Data<PathBuf>,
    req: &HttpRequest,
) -> Result<FilesResult> {
    let path: PathBuf = req.match_info().query("path").parse().unwrap();
    let path = &base_dir.join(path).to_path_buf();
    if !path.starts_with(base_dir.as_path()) {
        return Err(FileListInputError::InvalidPath(path.to_path_buf()).into());
    }
    let data = list_files(path, &base_dir).await?;
    debug!("files_listing: {:?}", data);
    Ok(data)
}

async fn list_files(dir: &PathBuf, base_dir: &PathBuf) -> Result<FilesResult> {
    let files = dir
        .read_dir()?
        .filter_map(|f| {
            f.ok().map(|f| FileInfo {
                name: f.file_name().into_string().unwrap(),
                is_dir: f.file_type().map(|t| t.is_dir()).unwrap_or(false),
            })
        })
        .filter(|f| !f.name.starts_with(".")) // ignore hidden files
        .collect::<Vec<_>>();

    Ok(FilesResult {
        files,
        path: relative_path(dir, base_dir)?,
        parent: dir
            .parent()
            .into_iter()
            .filter_map(|p| relative_path(&p.to_path_buf(), base_dir).ok())
            .next(),
    })
}

fn relative_path(path: &PathBuf, base_dir: &PathBuf) -> Result<String> {
    Ok(path
        .strip_prefix(base_dir)?
        .as_os_str()
        .to_str()
        .unwrap()
        .to_string())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    // Handlebars uses a repository for the compiled templates. This object must be
    // shared between the application threads, and is therefore passed to the
    // Application Builder as an atomic reference-counted pointer.

    let mut handlebars = Handlebars::new();
    handlebars.set_dev_mode(true);
    handlebars.register_helper("is-some-string", Box::new(is_some_string));
    handlebars
        .register_templates_directory(".hbs", "./templates")
        .unwrap();
    let handlebars_ref = web::Data::new(handlebars);

    let base_dir = PathBuf::from(dotenv::var("BASE_DIR").unwrap());
    let base_dir_data = web::Data::new(base_dir);
    HttpServer::new(move || {
        App::new()
            .wrap(tracing_actix_web::TracingLogger::default())
            .service(actix_files::Files::new("/static", "./static"))
            .app_data(base_dir_data.clone())
            .app_data(handlebars_ref.clone())
            .service(
                web::resource("/{path:.*}")
                    .route(
                        web::get()
                            .guard(actix_web::guard::Header("HX-Request", "true"))
                            .to(folder_contents),
                    )
                    .route(web::get().to(index)),
            )
        // .service(index)
        // .route("/{path:.*}", web::get().to(folder_contents))
        // .route("/{path:.*}", web::get().to(folder_contents))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

handlebars::handlebars_helper!(is_some_string: |option: Option<String>| option.is_some() );
