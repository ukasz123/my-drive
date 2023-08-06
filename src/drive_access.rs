use std::path::PathBuf;

use actix_multipart::form::tempfile::TempFile;
use anyhow::{Context, Result, Ok};

#[derive(Debug, serde::Serialize)]
pub(crate) struct FileType {
    pub mime: String,
    pub f_type: String,
}

impl Default for FileType {
    fn default() -> Self {
        Self {
            mime: "application/octet-stream".to_owned(),
            f_type: "unknown".to_owned(),
        }
    }
}

impl TryFrom<&std::path::Path> for FileType {
    type Error = anyhow::Error;
    fn try_from(value: &std::path::Path) -> Result<FileType> {
        let info = file_format::FileFormat::from_file(value);
        let info = info?;
        Ok(FileType {
            mime: info.media_type().to_owned(),
            f_type: match info.kind() {
                file_format::Kind::Application | file_format::Kind::Executable => "app",
                file_format::Kind::Archive
                | file_format::Kind::Compression
                | file_format::Kind::Disk
                | file_format::Kind::Package
                | file_format::Kind::Rom => "archive",
                file_format::Kind::Audio => "audio",
                file_format::Kind::Certificate
                | file_format::Kind::Document
                | file_format::Kind::Geospatial
                | file_format::Kind::Model => "document",
                file_format::Kind::Font => "font",
                file_format::Kind::Image => "image",
                file_format::Kind::Book
                | file_format::Kind::Subtitle
                | file_format::Kind::Syndication
                | file_format::Kind::Text => "txt",
                file_format::Kind::Playlist | file_format::Kind::Video => "video",
            }
            .to_owned(),
        })
    }
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub file_type: Option<FileType>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct FilesResult {
    pub files: Vec<FileInfo>,
    pub path: String,
    pub parent: Option<String>,
}

pub(crate) async fn list_files(dir: &PathBuf, base_dir: &PathBuf) -> Result<FilesResult> {
    let files = dir
        .read_dir()
        .context(format!("Reading {:?}", dir))?
        .filter_map(|f| {
            f.ok().map(|f| {
                let is_dir = f.file_type().map(|t| t.is_dir()).unwrap_or(false);
                FileInfo {
                    name: f.file_name().into_string().unwrap(),
                    is_dir: is_dir,
                    file_type: if is_dir {
                        None
                    } else {
                        Some((f.path().as_path()).try_into().unwrap_or_default())
                    },
                }
            })
        })
        .filter(|f| !f.name.starts_with(".")) // ignore hidden files
        .collect::<Vec<_>>();

    Ok(FilesResult {
        files,
        path: relative_path(dir, base_dir)?,
        parent: dir
            .parent()
            .into_iter()
            .filter_map(|p| relative_path(&p.to_path_buf(), base_dir).ok())
            .next(),
    })
}

fn relative_path(path: &PathBuf, base_dir: &PathBuf) -> Result<String> {
    let path = path.strip_prefix(base_dir)?.as_os_str().to_str().unwrap();
    if path.is_empty() {
        return Ok("".to_owned());
    }
    Ok(format!("/{}", path))
}

pub(crate) fn query_files(query: &str, base_dir: &PathBuf) -> Result<Vec<FileInfo>> {
    use glob::glob;
    let paths = glob(&format!(
        "{}/**/{}*",
        &base_dir.as_os_str().to_str().unwrap(),
        query
    ))?;

    let files = paths
        .filter_map(|p| p.ok())
        .map(|path| {
            let is_dir = path.is_dir();
            FileInfo {
                name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                is_dir: is_dir,
                file_type: if is_dir {
                    None
                } else {
                    Some((path.as_path()).try_into().unwrap_or_default())
                },
            }
        })
        .filter(|f| !f.name.starts_with(".")) // ignore hidden files
        .collect::<Vec<_>>();
    Ok(files)
}

pub(crate) fn save_files(files: Vec<TempFile>, dir: &PathBuf) -> impl Iterator<Item = (String, Result<std::fs::File>)> +'_ {
    files
        .into_iter()
        .filter(|file| file.file_name.is_some())
        .map(|file| {
            let name = file.file_name.unwrap();
            let path = dir.join(&name);
            let persist_result = file.file.persist(path).context("Persisting file");
            (name, persist_result)
        })
}

pub(crate) fn delete_file(path: &PathBuf) -> Result<()> {
    std::fs::remove_file(path).context(format!("Deleting {:?}", path))
}

pub(crate) fn create_dir(new_dir_path: &PathBuf) -> Result<()> {
    std::fs::create_dir(new_dir_path).context(format!("Creating directory {:?}", new_dir_path))
}
