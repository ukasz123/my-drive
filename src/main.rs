mod drive_access;
mod handlebars_utils;
mod server;
mod telemetry;
mod webservices;

#[cfg(not(feature = "ngrok"))]
mod default_runner;

#[cfg(feature = "ngrok")]
mod ngrok_runner;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let tracing_subscriber = telemetry::create_subscriber();

    telemetry::init_telemetry(tracing_subscriber);

    #[cfg(not(feature = "ngrok"))]
    use default_runner::run_server;

    #[cfg(feature = "ngrok")]
    use ngrok_runner::run_server;

    run_server().await?;

    Ok(())
}
