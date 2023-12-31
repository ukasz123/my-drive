use actix_web::http::header;

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
        let body = if let Some(_) = req.headers().get("HX-Request") {
            let body = self.hb.render(self.template, &self.data).unwrap();
            actix_web::Either::Left(actix_web::HttpResponse::Ok()
            .insert_header(header::ContentType::html())
            .body(body))
        } else {
            actix_web::Either::Right(actix_web::web::Json(&self.data))
        };
        body.respond_to(req)
    }
}
