use std::path::PathBuf;

use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use glob::glob;
use handlebars::{handlebars_helper, Handlebars};
use serde_json::json;
use tracing::debug;

#[derive(Debug, serde::Serialize)]
struct FileType {
    pub mime: String,
    pub f_type: String,
}

impl Default for FileType {
    fn default() -> Self {
        Self {
            mime: "application/octet-stream".to_owned(),
            f_type: "unknown".to_owned(),
        }
    }
}

impl TryFrom<&std::path::Path> for FileType {
    type Error = anyhow::Error;
    fn try_from(value: &std::path::Path) -> Result<FileType> {
        let info = file_format::FileFormat::from_file(value);
        let info = info?;
        Ok(FileType {
            mime: info.media_type().to_owned(),
            f_type: match info.kind() {
                file_format::Kind::Application | file_format::Kind::Executable => "app",
                file_format::Kind::Archive
                | file_format::Kind::Compression
                | file_format::Kind::Disk
                | file_format::Kind::Package
                | file_format::Kind::Rom => "archive",
                file_format::Kind::Audio => "audio",
                file_format::Kind::Certificate
                | file_format::Kind::Document
                | file_format::Kind::Geospatial
                | file_format::Kind::Model => "document",
                file_format::Kind::Font => "font",
                file_format::Kind::Image => "image",
                file_format::Kind::Book
                | file_format::Kind::Subtitle
                | file_format::Kind::Syndication
                | file_format::Kind::Text => "txt",
                file_format::Kind::Playlist | file_format::Kind::Video => "video",
            }
            .to_owned(),
        })
    }
}

#[derive(Debug, serde::Serialize)]
struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub file_type: Option<FileType>,
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
    // debug!("files_listing: {:?}", data);
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
            f.ok().map(|f| {
                let is_dir = f.file_type().map(|t| t.is_dir()).unwrap_or(false);
                FileInfo {
                    name: f.file_name().into_string().unwrap(),
                    is_dir: is_dir,
                    file_type: if is_dir {
                        None
                    } else {
                        Some((f.path().as_path()).try_into().unwrap_or_default())
                    },
                }
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
    let path = path.strip_prefix(base_dir)?.as_os_str().to_str().unwrap();
    if path.is_empty() {
        return Ok("".to_owned());
    }
    Ok(format!("/{}", path))
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
                .filter(|f| !f.name.starts_with("."))// ignore hidden files
                .collect::<Vec<_>>();
                let body = hb.render("query_results", &json!({"files" : files})).unwrap();
                HttpResponse::Ok().body(body)
        }
        Err(_) => {
            HttpResponse::InternalServerError().finish()
        }
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

    let mut handlebars = Handlebars::new();
    handlebars.set_dev_mode(true);
    handlebars.register_helper("is-some-string", Box::new(is_some_string));
    handlebars.register_decorator("switch", Box::new(switch));
    handlebars.register_helper("case", Box::new(case));
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
            .service(query_files)
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

use handlebars::*;
fn switch<'reg: 'rc, 'rc>(
    d: &Decorator,
    _: &Handlebars,
    ctx: &Context,
    rc: &mut RenderContext,
) -> Result<(), RenderError> {
    let switch_param = d
        .param(0)
        .ok_or(RenderError::new("switch param not found"))?;
    // modify json object
    let mut new_ctx = ctx.clone();
    {
        let new_value = switch_param.value().clone();
        println!("new_value: {:?}", new_value);
        let data = new_ctx.data_mut();
        if let Some(ref mut m) = data.as_object_mut() {
            m.insert("my-drive-switch".to_string(), new_value);
        }
    }
    rc.set_context(new_ctx);
    Ok(())
}

fn case<'reg, 'rc>(
    h: &Helper<'reg, 'rc>,
    r: &'reg Handlebars<'reg>,
    ctx: &'rc Context,
    rc: &mut RenderContext<'reg, 'rc>,
    out: &mut dyn Output,
) -> HelperResult {
    let actual = ctx.data();
    let expected = h.param(0).unwrap().value();
    debug!("case: {:?} == {:?}", actual, expected);
    if expected == actual {
        h.template()
            .map(|t| {
                let v = h.param(0).unwrap().value();
                rc.set_context(Context::from(v.clone()));
                t.render(r, ctx, rc, out)
            })
            .unwrap_or(Ok(()))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use handlebars::Handlebars;

    #[test]
    fn test_handlebars_case() {
        let mut handlebars = Handlebars::new();
        handlebars.register_decorator("switch", Box::new(super::switch));
        handlebars.register_helper("case", Box::new(super::case));
        let template = "{{#*switch test}}>{{my-drive-switch}}<{{#if (eq my-drive-switch 1)}}one{{/if}}{{#if (eq my-drive-switch 2)}}2{{/if}}{{#case 2}}two{{/case}}{{#case 3}}three{{/case}}{{/switch}}";
        assert_eq!(
            handlebars
                .render_template(template, &serde_json::json!({"test":1}))
                .unwrap(),
            "one".to_owned()
        );
        assert_eq!(
            handlebars
                .render_template(template, &serde_json::json!({"test":2}))
                .unwrap(),
            "two".to_owned()
        );
    }
}
