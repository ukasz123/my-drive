use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use actix_multipart::form::tempfile::TempFile;
use anyhow::{Context, Ok, Result};
use glob::MatchOptions;

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
                file_format::Kind::Executable => "app",
                file_format::Kind::Archive
                | file_format::Kind::Compressed
                | file_format::Kind::Disk
                | file_format::Kind::Database
                | file_format::Kind::Package
                | file_format::Kind::Rom => "archive",
                file_format::Kind::Audio => "audio",
                file_format::Kind::Geospatial
                | file_format::Kind::Spreadsheet
                | file_format::Kind::Formula
                | file_format::Kind::Diagram
                | file_format::Kind::Metadata
                | file_format::Kind::Presentation
                | file_format::Kind::Model
                | file_format::Kind::Other => "document",
                file_format::Kind::Font => "font",
                file_format::Kind::Image => "image",
                file_format::Kind::Ebook
                | file_format::Kind::Document
                | file_format::Kind::Subtitle => "txt",

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
    pub metadata: Option<FileMetadata>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct FileMetadata {
    pub(crate) created_at: Option<u64>,
    pub(crate) modified_at: Option<u64>,
    pub(crate) size: Option<u64>,
}

impl PartialEq for FileInfo {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.is_dir == other.is_dir
    }
}

impl Eq for FileInfo {}

impl PartialOrd for FileInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for FileInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.is_dir.cmp(&other.is_dir) {
            std::cmp::Ordering::Equal => self.name.cmp(&other.name),
            ord => ord,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct FilesResult {
    pub files: Vec<FileInfo>,
    pub path: String,
    pub parent: Option<String>,
}

fn to_file_metadata(metadata: std::fs::Metadata) -> FileMetadata {
    FileMetadata {
        created_at: metadata
            .created()
            .ok()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
        modified_at: metadata
            .modified()
            .ok()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
        size: Some(metadata.size()),
    }
}

#[tracing::instrument]
pub(crate) async fn list_files(dir: &PathBuf, base_dir: &PathBuf) -> Result<FilesResult> {

    let mut files = dir
        .read_dir()
        .context(format!("Reading {:?}", dir))?
        .filter_map(|f| {
            f.ok().map(|f| {
                let is_dir = f.file_type().map(|t| t.is_dir()).unwrap_or(false);
                FileInfo {
                    name: f.file_name().into_string().unwrap(),
                    is_dir,
                    file_type: if is_dir {
                        None
                    } else {
                        Some((f.path().as_path()).try_into().unwrap_or_default())
                    },
                    metadata: f.metadata().ok().map(to_file_metadata),
                }
            })
        })
        .filter(|f| !f.name.starts_with('.')) // ignore hidden files
        .collect::<Vec<_>>();

    files.sort();
    files.reverse();

    Ok(FilesResult {
        files,
        path: relative_path(dir, base_dir)?,
        parent: dir
            .parent()
            .into_iter()
            .filter_map(|p| relative_path(p, base_dir).ok())
            .next(),
    })
}

fn relative_path(path: &Path, base_dir: &PathBuf) -> Result<String> {
    let path = path.strip_prefix(base_dir)?.as_os_str().to_str().unwrap();
    if path.is_empty() {
        return Ok("".to_owned());
    }
    Ok(format!("/{}", path))
}

#[tracing::instrument]
pub(crate) fn query_files(query: &str, base_dir: &Path) -> Result<Vec<FileInfo>> {
    use glob::glob_with;
    let paths = glob_with(
        &format!("{}/**/{}*", &base_dir.as_os_str().to_str().unwrap(), query),
        MatchOptions {
            case_sensitive: false,
            ..Default::default()
        },
    )?;

    let mut files = paths
        .filter_map(|p| p.ok())
        .map(|path| {
            let is_dir = path.is_dir();
            FileInfo {
                name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                is_dir,
                file_type: if is_dir {
                    None
                } else {
                    Some((path.as_path()).try_into().unwrap_or_default())
                },
                metadata: path.metadata().ok().map(to_file_metadata),
            }
        })
        .filter(|f| !f.name.starts_with('.')) // ignore hidden files
        .collect::<Vec<_>>();
    files.sort();
    files.reverse();
    Ok(files)
}

#[tracing::instrument]
pub(crate) fn save_files(
    files: Vec<TempFile>,
    dir: &Path,
) -> impl Iterator<Item = (String, Result<std::fs::File>)> + '_ {
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

#[tracing::instrument]
pub(crate) fn delete_file_or_directory(path: &PathBuf) -> Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path).context(format!("Deleting directory {:?}", path))
    } else {
        std::fs::remove_file(path).context(format!("Deleting file {:?}", path))
    }
}

#[tracing::instrument]
pub(crate) fn create_dir(new_dir_path: &PathBuf) -> Result<()> {
    std::fs::create_dir(new_dir_path).context(format!("Creating directory {:?}", new_dir_path))
}
