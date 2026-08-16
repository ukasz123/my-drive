use std::path::PathBuf;

use crate::utils::zip::zip_dir_to_sink;
use actix_web::{
    http::StatusCode,
    web::{self, Bytes},
    Either, HttpResponse, HttpResponseBuilder, Responder,
};
use futures::{stream_select, FutureExt};
use tokio_stream::StreamExt as _;
use tokio_util::{compat::TokioAsyncWriteCompatExt, io::ReaderStream};
use tracing::{debug_span, instrument, Instrument, Span};

#[instrument(name = "zip_dir")]
pub(super) async fn handle(path: web::ReqData<crate::server::RequestedPath>) -> impl Responder {
    let absolute_path: PathBuf = path.as_ref().clone();
    let is_dir = absolute_path.is_dir();
    if !is_dir {
        return Either::Left(HttpResponse::BadRequest().finish());
    }

    let (stream, sink) = tokio::io::simplex(16 * 1024);
    let (error_tx, error_rx) = tokio::sync::oneshot::channel();
    let zip_mime = actix_files::file_extension_to_mime("zip");

    let stream = ReaderStream::new(stream);
    let handler_span = Span::current();

    let _handle = tokio::spawn(async move {
        let task_span = debug_span!("zip_task");
        let sink = sink.compat_write();
        task_span.follows_from(&handler_span);
        let result = zip_dir_to_sink(&absolute_path, sink)
            .instrument(task_span)
            .await;
        match result {
            Ok(_entry) => {
                let _ = error_tx.send(None);
            }
            Err(err) => {
                let _ = error_tx.send(Some(err));
            }
        };
    });
    let error_stream = error_rx
        .into_stream()
        .filter_map(|read_res| read_res.ok().flatten())
        .map(Result::<Bytes, std::io::Error>::Err);
    let combined_stream = stream_select!(stream, error_stream);

    let resp_stream = HttpResponseBuilder::new(StatusCode::OK)
        .content_type(zip_mime.to_string())
        .streaming(combined_stream);

    Either::Right(resp_stream)
}
