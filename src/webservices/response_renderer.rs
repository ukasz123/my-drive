use actix_web::http::header;
use tracing::trace_span;

#[derive(Debug)]
pub(super) struct ResponseRenderer<'a, T: serde::ser::Serialize> {
    data: T,
    template: &'static str,
    hb: std::sync::Arc<handlebars::Handlebars<'a>>,
}

impl<'a, T: serde::ser::Serialize> ResponseRenderer<'a, T> {
    pub fn new(
        data: T,
        template: &'static str,
        hb: std::sync::Arc<handlebars::Handlebars<'a>>,
    ) -> Self {
        Self { data, template, hb }
    }
}

impl<'a, T> actix_web::Responder for ResponseRenderer<'a, T>
where
    T: serde::ser::Serialize,
{
    type Body =
        actix_web::body::EitherBody<actix_web::body::BoxBody, actix_web::body::EitherBody<String>>;
    
    fn respond_to(self, req: &actix_web::HttpRequest) -> actix_web::HttpResponse<Self::Body> {
        let span = trace_span!("responsive_render_content");
        let _enter = span.enter();
        let body = if req.headers().get("HX-Request").is_some()
            || !req.headers().get("Accept").is_some_and(|accept_header| {
                accept_header
                    .to_str()
                    .is_ok_and(|accept_header_val| accept_header_val.contains("json"))
            }) {
            let body = self.hb.render(self.template, &self.data).unwrap();
            actix_web::Either::Left(
                actix_web::HttpResponse::Ok()
                    .insert_header(header::ContentType::html())
                    .body(body),
            )
        } else {
            actix_web::Either::Right(actix_web::web::Json(&self.data))
        };
        body.respond_to(req)
    }
}
