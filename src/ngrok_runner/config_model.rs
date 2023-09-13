use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct NgrokConfig {
    pub(crate) authoken: String,
    pub(crate) domain: Option<String>,
    pub(crate) oauth: Option<Oauth>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Oauth {
    pub(crate) provider: String,
    pub(crate) allowed_emails: Option<Vec<String>>,
    pub(crate) allowed_domains: Option<Vec<String>>,
}
