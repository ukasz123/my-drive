use std::{
    future::{ready, Future, Ready},
    path::PathBuf,
    pin::Pin,
};

use actix_web::{
    body::EitherBody,
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use tracing::debug;

use crate::webservices::FileListInputError;

pub(crate) struct RequestPath;

impl<S, B> Transform<S, ServiceRequest> for RequestPath
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestPathMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestPathMiddleware { service }))
    }
}

pub(crate) struct RequestPathMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestPathMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let base_dir = req.app_data::<actix_web::web::Data<PathBuf>>().unwrap();
        let path: PathBuf = req.match_info().query("path").parse().unwrap();
        let combined_path = base_dir.join(&path).to_path_buf();
        let is_path_valid = combined_path.starts_with(base_dir.as_path());

        if !is_path_valid {
            debug!("Invalid path: {:?}", &path);
            return Box::pin(async move {
                actix_web::Result::Ok(
                    req.into_response(
                        HttpResponse::BadRequest()
                            .body(FileListInputError::InvalidPath(path.to_path_buf()).to_string()),
                    )
                    .map_into_right_body(),
                )
            });
        }
        req.extensions_mut().insert(RequestedPath(combined_path));
        let r = self.service.call(req);

        Box::pin(async move {
            // forwarded responses map to "left" body
            r.await.map(ServiceResponse::map_into_left_body)
        })
    }

    dev::forward_ready!(service);
}
#[derive(Debug, Clone)]
pub(crate) struct RequestedPath(PathBuf);

impl Into<PathBuf> for RequestedPath {
    fn into(self) -> PathBuf {
        self.0
    }
}

impl AsRef<PathBuf> for RequestedPath {
    fn as_ref(&self) -> &PathBuf {
        &self.0
    }
}
