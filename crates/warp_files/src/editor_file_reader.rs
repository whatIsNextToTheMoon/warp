use std::io;
use std::path::Path;

use futures::io::AsyncReadExt;
use warp_util::file::FileLoadError;

const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_LINES: usize = 100_000;
const MAX_LINE_BYTES: usize = 16 * 1024;

/// Bounds the work required to construct a full editor buffer and its layout.
pub fn validate_editor_content(content: &str) -> Result<(), FileLoadError> {
    if content.len() > MAX_BYTES {
        return Err(FileLoadError::EditorFileTooLarge);
    }
    for (index, line) in content.split('\n').enumerate() {
        if index >= MAX_LINES {
            return Err(FileLoadError::EditorTooManyLines);
        }
        if line.len() > MAX_LINE_BYTES {
            return Err(FileLoadError::EditorLineTooLong);
        }
    }
    Ok(())
}

pub async fn read_content_for_editor(path: &Path) -> Result<String, FileLoadError> {
    let metadata = async_fs::metadata(path).await?;
    if !metadata.is_file() {
        return Err(FileLoadError::NotRegularFile);
    }
    if metadata.len() > MAX_BYTES as u64 {
        return Err(FileLoadError::EditorFileTooLarge);
    }

    // Limit the read as well: a file can grow after the metadata check.
    let file = async_fs::File::open(path).await?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .await?;
    if bytes.len() > MAX_BYTES {
        return Err(FileLoadError::EditorFileTooLarge);
    }
    let content = String::from_utf8(bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    validate_editor_content(&content)?;
    Ok(content)
}

#[cfg(test)]
#[path = "editor_file_reader_tests.rs"]
mod tests;
