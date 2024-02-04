use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::drive_access::FilesResult;
use actix_files::NamedFile;
use actix_web::Either;

pub(super) async fn list_files_or_file_contents(
    path: &PathBuf,
    base_dir: &PathBuf,
) -> Result<Either<FilesResult, NamedFile>> {
    if path.is_file() {
        let file = NamedFile::open(path).context("Could not open file")?;
        return Ok(Either::Right(file));
    }
    let data = crate::drive_access::list_files(&path, &base_dir).await?;
    Ok(Either::Left(data))
}
