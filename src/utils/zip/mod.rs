use std::path::{Path, PathBuf};

use async_zip::{tokio::write::ZipFileWriter, ZipEntryBuilder, ZipString};
use futures::AsyncWriteExt as _;
use tokio::io::AsyncWrite;
use tokio_util::compat::FuturesAsyncWriteCompatExt as _;
use tracing::{debug_span, instrument, Instrument as _};

#[instrument(err(Debug), skip(sink))]
pub(crate) async fn zip_dir_to_sink<W: AsyncWrite + Unpin>(
    dir_path: &PathBuf,
    sink: tokio_util::compat::Compat<W>,
) -> Result<(), std::io::Error> {
    let mut zip_file_writer = ZipFileWriter::new(sink);

    let walker = walkdir::WalkDir::new(dir_path);
    for e in walker.into_iter() {
        let entry = e.map_err(|err| std::io::Error::new(std::io::ErrorKind::NotConnected, err))?;
        let entry_path = entry.path();
        if entry_path.is_file() {
            let entry_path_stripped = entry_path
                .strip_prefix(dir_path)
                .expect("walker should never move outside of the subtree");
            let entry_path_str = entry_path_stripped
                .to_str()
                .expect("path contains non-UTF-8 characters");
            let span = debug_span!("zip_file", path = entry_path_str);
            compress_file(entry_path_str, entry_path, &mut zip_file_writer)
                .instrument(span)
                .await?;
        }
    }
    let mut closed = zip_file_writer
        .close()
        .await
        .map_err(std::io::Error::other)?;
    closed.close().await?;
    Ok(())
}

pub(crate) async fn compress_file<W: AsyncWrite + Unpin>(
    entry_path_str: impl Into<ZipString>,
    entry_path: &Path,
    zip_file_writer: &mut ZipFileWriter<W>,
) -> Result<(), std::io::Error> {
    let file_entry = ZipEntryBuilder::new(entry_path_str.into(), async_zip::Compression::Deflate);
    let file_entry_writer = zip_file_writer
        .write_entry_stream(file_entry)
        .await
        .map_err(std::io::Error::other)?;
    let mut file = tokio::fs::File::open(entry_path).await?;
    let mut entry_writer = file_entry_writer.compat_write();
    let _written = tokio::io::copy(&mut file, &mut entry_writer).await?;
    entry_writer
        .into_inner()
        .close()
        .await
        .map_err(std::io::Error::other)?;
    Ok(())
}
