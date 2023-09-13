use std::path::PathBuf;

use actix_files::NamedFile;
use actix_multipart::form::text::Text;
use actix_web::{web, App, Either, HttpResponse, HttpServer, Responder};
use anyhow::{Context, Result};
use drive_access::FilesResult;
use handlebars::Handlebars;
use serde_json::json;

mod drive_access;
mod handlebars_utils;
mod server;

async fn index(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    path: web::ReqData<server::RequestedPath>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let path = path.into_inner().into();
    let data = handle_list_files_request(&path, &base_dir).await;
    match data {
        Ok(data) => match data {
            Either::Left(data) => {
                let body = hb.render("index", &data).unwrap();
                HttpResponse::Ok().body(body)
            }
            Either::Right(resp) => resp.into_response(&req),
        },
        Err(anyhow_err) => match anyhow_err.downcast_ref::<FileListInputError>() {
            Some(err) => HttpResponse::BadRequest().body(err.to_string()),
            None => HttpResponse::NotFound().finish(),
        },
    }
}

async fn folder_contents(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    path: web::ReqData<server::RequestedPath>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let path = path.into_inner().into();
    let data = handle_list_files_request(&path, &base_dir).await;
    match data {
        Ok(data) => match data {
            Either::Left(data) => {
                let body = hb.render("files_listing", &data).unwrap();
                HttpResponse::Ok().body(body)
            }
            Either::Right(resp) => resp.into_response(&req),
        },
        Err(anyhow_err) => match anyhow_err.downcast_ref::<FileListInputError>() {
            Some(err) => HttpResponse::BadRequest().body(err.to_string()),
            None => HttpResponse::NotFound().finish(),
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
) -> Result<Either<FilesResult, NamedFile>> {
    if path.is_file() {
        let file = NamedFile::open(path).context("Could not open file")?;
        return Ok(Either::Right(file));
    }
    let data = drive_access::list_files(&path, &base_dir).await?;
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

    let files = drive_access::query_files(query, base_dir.as_ref());
    match files {
        Ok(files) => {
            let body = hb
                .render("query_results", &json!({ "files": files }))
                .unwrap();
            HttpResponse::Ok().body(body)
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Error querying files: {}", e)),
    }
}

#[derive(Debug, actix_multipart::form::MultipartForm)]
struct UploadOrNewDirForm {
    #[multipart(rename = "file")]
    files: Vec<actix_multipart::form::tempfile::TempFile>,

    #[multipart(rename = "new_folder")]
    new_folder_name: Option<Text<String>>,
}

async fn upload_file_or_new_dir(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    form: actix_multipart::form::MultipartForm<UploadOrNewDirForm>,
    path: web::ReqData<server::RequestedPath>,
) -> impl Responder {
    let path = path.as_ref();
    let dir_path = base_dir.join(path).to_path_buf();
    // create new folder
    if let Some(new_dir_name) = &form.new_folder_name {
        let new_dir_path = dir_path.join(new_dir_name.as_str());
        let data = drive_access::create_dir(&new_dir_path);
        return match data {
            Ok(_) => {
                let data = drive_access::list_files(&dir_path, &base_dir).await;
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
    // save new files
    let files = form.into_inner().files;
    let results = drive_access::save_files(files, &dir_path);
    let summary = results
        .map(|(name, r)| match r {
            Ok(_) => {
                format!("{} file saved", name)
            }
            Err(e) => {
                format!("{} file failed to save: {}", name, e)
            }
        })
        .map(|message| format!("<li>{}</li>", message))
        .collect::<String>();
    let data = drive_access::list_files(&dir_path, &base_dir).await;
    match data {
        Ok(data) => {
            let body = hb.render("files_listing", &data).unwrap();
            let summary = format!("<ul>{}</ul>", summary);
            let confirmation_toast = hb
                .render("confirmation_toast", &json!({ "message": summary }))
                .unwrap();
            HttpResponse::Ok().body(format!("{}{}", body, confirmation_toast))
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn delete_file(
    hb: web::Data<Handlebars<'_>>,
    base_dir: web::Data<PathBuf>,
    path: web::ReqData<server::RequestedPath>,
) -> impl Responder {
    let path = path.as_ref();
    let dir_path = base_dir.join(path).to_path_buf();
    let data = drive_access::delete_file(&dir_path);
    match data {
        Ok(_) => {
            let data =
                drive_access::list_files(&dir_path.parent().unwrap().to_path_buf(), &base_dir)
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    run_server().await?;

    Ok(())
}

#[cfg(not(feature = "ngrok"))]
async fn run_server() -> std::io::Result<()> {
    let local_address = (
        "127.0.0.1",
        dotenv::var("PORT")
            .unwrap_or("8080".to_owned())
            .parse::<u16>()
            .unwrap(),
    );

    let server = start_http_server(&local_address);
    server.await
}

#[cfg(feature = "ngrok")]
async fn run_server() -> std::io::Result<()> {
    use futures::pin_mut;

    let local_address = (
        "127.0.0.1",
        dotenv::var("PORT")
            .unwrap_or("8080".to_owned())
            .parse::<u16>()
            .unwrap(),
    );

    let server = start_http_server(&local_address);
    let forwarding = start_ngrok(&local_address);

    pin_mut!(server);
    pin_mut!(forwarding);

    futures::future::select(server, forwarding).await;

    Ok(())
}

async fn start_http_server(local_address: &impl std::net::ToSocketAddrs) -> std::io::Result<()> {
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
                    .route(web::put().to(upload_file_or_new_dir))
                    .route(web::delete().to(delete_file)),
            )
    })
    .bind(local_address)?
    .run()
    .await
}
#[cfg(feature = "ngrok")]
mod config_model;

#[cfg(feature = "ngrok")]
async fn start_ngrok(local_address: &(&str, u16)) -> anyhow::Result<()> {
    use ngrok::prelude::*;
    use tracing::{warn, info};

    use crate::config_model::NgrokConfig;

    let ngrok_config = std::fs::read_to_string("ngrok-config.toml")
        .context("Failed to read ngrok-config.toml")
        .and_then(|contents| {
            toml::from_str::<NgrokConfig>(&contents).context("Failed to parse ngrok-config.toml")
        })
        .unwrap_or_else(|e| {
            warn!(
                "Failed to read ngrok-config.toml, using environment variables: {:?}",
                e
            );
            let ngrok_auth_token = dotenv::var("NGROK_AUTH_TOKEN").unwrap();
            let ngrok_domain = dotenv::var("NGROK_DOMAIN").ok();
            NgrokConfig {
                authoken: ngrok_auth_token,
                domain: ngrok_domain,
                oauth: None,
            }
        });
    let mut tun_builder = ngrok::Session::builder()
        // Set the auth token
        .authtoken(ngrok_config.authoken)
        // Connect the ngrok session
        .connect()
        .await?
        // Start a tunnel with an HTTP edge
        .http_endpoint();
    if let Some(domain) = ngrok_config.domain {
        tun_builder = tun_builder.domain(domain);
    };

    if let Some(oauth) = ngrok_config.oauth {
        let mut oauth_options = ngrok::config::OauthOptions::new(oauth.provider);
        if let Some(allowed_emails) = oauth.allowed_emails {
            for email in allowed_emails {
                oauth_options = oauth_options.allow_email(email);
            }
        }
        if let Some(allowed_domains) = oauth.allowed_domains {
            for domain in allowed_domains {
                oauth_options = oauth_options.allow_domain(domain);
            }
        }
        tun_builder = tun_builder.oauth(oauth_options);
    }

    let mut tun = tun_builder.listen().await?;
    info!("Tunnel started on URL: {:?}", tun.url());
    Ok(tun.forward_tcp(local_address).await?)
}
