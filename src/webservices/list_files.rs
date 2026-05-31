use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::{debug_span, instrument, Instrument, Span};

use crate::drive_access::FilesResult;
use actix_files::NamedFile;
use actix_web::Either;

#[instrument]
pub(super) async fn list_files_or_file_contents(
    path: &PathBuf,
    base_dir: &PathBuf,
) -> Result<Either<FilesResult, NamedFile>> {
    if path.is_file() {
        let file = debug_span!("named_file_open")
            .in_scope(|| NamedFile::open(path))
            .context("Could not open file")?;
        return Ok(Either::Right(file));
    }
    let data = crate::drive_access::list_files(path, base_dir)
        .instrument(Span::current())
        .await?;
    Ok(Either::Left(data))
}
