use std::path::PathBuf;

use actix_web::{guard, web, App, HttpServer};
use anyhow::Context;


mod create_dir;
mod delete_file;
mod folder_contents;
mod index;
mod list_files;
mod query_files;
mod response_renderer;
mod upload_file;
mod utilities;

#[derive(Debug, thiserror::Error)]
pub(crate) enum FileListInputError {
    #[error("Invalid path: {0:?}")]
    InvalidPath(PathBuf),
}

/// Starts HTTP server.
pub(crate) async fn start_http_server(
    local_address: &impl std::net::ToSocketAddrs,
) -> anyhow::Result<()> {
    // Handlebars uses a repository for the compiled templates. This object must be
    // shared between the application threads, and is therefore passed to the
    // Application Builder as an atomic reference-counted pointer.

    let handlebars = crate::handlebars_utils::prepare();
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
                web::resource("/")
                    .guard(guard::Post())
                    .route(web::post().to(query_files::handle)),
            )
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
                            .to(folder_contents::handle),
                    )
                    .route(web::get().to(index::handle))
                    .route(
                        web::put()
                            .guard(guard::Header("command", "new_folder"))
                            .to(create_dir::handle),
                    )
                    .route(web::put().to(upload_file::handle))
                    .route(web::delete().to(delete_file::handle)),
            )
    })
    .bind(local_address)?
.run()
.await
.context("Cannot run the server")
}
