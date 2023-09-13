use anyhow::Context;

pub(crate) async fn run_server() -> std::io::Result<()> {
    use futures::pin_mut;

    let local_address = (
        "127.0.0.1",
        dotenv::var("PORT")
            .unwrap_or("8080".to_owned())
            .parse::<u16>()
            .unwrap(),
    );

    let server = crate::webservices::start_http_server(&local_address);
    let forwarding = start_ngrok(&local_address);

    pin_mut!(server);
    pin_mut!(forwarding);

    futures::future::select(server, forwarding).await;

    Ok(())
}

mod config_model;

async fn start_ngrok(local_address: &(&str, u16)) -> anyhow::Result<()> {
    use ngrok::prelude::*;
    use tracing::{warn, info};

    use config_model::NgrokConfig;

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
