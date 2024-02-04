use tracing::info;

pub(crate) async fn run_server() -> std::io::Result<()> {
    let local_address = (
        "0.0.0.0",
        dotenv::var("PORT")
            .unwrap_or("8080".to_owned())
            .parse::<u16>()
            .unwrap(),
    );
    info!("Starting server at {:?}", local_address);
    let server = crate::webservices::start_http_server(&local_address);
    server.await
}
