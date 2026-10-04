//! Bounded Markdown IO and managed local document library.
use document_core::MAX_DOCUMENT_BYTES;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub mod library;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    InvalidPath,
    TooLarge,
    InvalidUtf8,
    ExternalChange,
    Busy,
    UnsafeTarget,
    InvalidProject,
    ProjectLimit,
}
impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Io(_) => "文件操作失败，请检查路径、目录和权限",
            Self::InvalidPath => "请输入绝对 Markdown/TXT 文件路径",
            Self::TooLarge => "文件超过 1 MiB 限制",
            Self::InvalidUtf8 => "文件不是有效的 UTF-8 文本",
            Self::ExternalChange => "磁盘文件已变化或目标已存在；请先打开，不会覆盖",
            Self::Busy => "另一个保存正在进行，或存在遗留锁文件",
            Self::UnsafeTarget => "目标必须是普通文件，不能是符号链接或目录",
            Self::InvalidProject => "不是兼容的 DocGenie 项目，或项目状态损坏",
            Self::ProjectLimit => "项目或素材超过限额",
        })
    }
}
impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}
impl From<io::Error> for StoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

fn validate(path: &Path) -> Result<(), StoreError> {
    if !path.is_absolute()
        || path.file_name().is_none()
        || !path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("txt"))
    {
        return Err(StoreError::InvalidPath);
    }
    Ok(())
}

pub fn load(path: &Path) -> Result<Option<String>, StoreError> {
    validate(path)?;
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => {
            return Err(StoreError::UnsafeTarget);
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let file = File::open(path)?;
    let mut bytes = Vec::new();
    file.take(MAX_DOCUMENT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(StoreError::TooLarge);
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| StoreError::InvalidUtf8)
}

/// Cooperating writers serialize via a sidecar lock. The original target must
/// exactly match the loaded baseline (None = create, never overwrite).
/// Non-cooperating editors can race the final check; this is not a database CAS.
pub fn save(path: &Path, text: &str, expected: Option<&str>) -> Result<(), StoreError> {
    validate(path)?;
    if text.len() > MAX_DOCUMENT_BYTES {
        return Err(StoreError::TooLarge);
    }
    let parent = path.parent().ok_or(StoreError::InvalidPath)?;
    let name = path
        .file_name()
        .ok_or(StoreError::InvalidPath)?
        .to_string_lossy();
    let lock_path = parent.join(format!(".{name}.agent-docs.lock"));
    let lock_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                StoreError::Busy
            } else {
                error.into()
            }
        })?;
    let _lock = RemoveOnDrop(lock_path);
    // Hold the lock handle until all persistence steps have finished.
    let _lock_file = lock_file;
    if load(path)?.as_deref() != expected {
        return Err(StoreError::ExternalChange);
    }
    let temp = parent.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let _temp = RemoveOnDrop(temp.clone());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o777)
            .unwrap_or(0o600);
        file.set_permissions(fs::Permissions::from_mode(mode))?;
    }
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    drop(file);
    // Re-check after writing the temporary file before atomic replacement.
    if load(path)?.as_deref() != expected {
        return Err(StoreError::ExternalChange);
    }
    fs::rename(&temp, path)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

struct RemoveOnDrop(PathBuf);
impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
