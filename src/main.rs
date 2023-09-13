

mod drive_access;
mod handlebars_utils;
mod server;
mod webservices;

#[cfg(not(feature = "ngrok"))]
mod default_runner;

#[cfg(feature = "ngrok")]
mod ngrok_runner;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    #[cfg(not(feature = "ngrok"))]
    use default_runner::run_server;

    #[cfg(feature = "ngrok")]
    use ngrok_runner::run_server;

    run_server().await?;

    Ok(())
}
