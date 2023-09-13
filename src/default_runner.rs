pub(crate) async fn run_server() -> std::io::Result<()> {
    let local_address = (
        "127.0.0.1",
        dotenv::var("PORT")
            .unwrap_or("8080".to_owned())
            .parse::<u16>()
            .unwrap(),
    );

    let server = crate::webservices::start_http_server(&local_address);
    server.await
}
