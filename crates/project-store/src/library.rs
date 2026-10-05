//! Managed local Markdown files. Markdown is canonical; hidden JSON sidecars
//! retain local comments/undo only when their document text matches the file.
use crate::StoreError;
use document_core::{Workbench, WorkbenchSnapshot};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

const MAX_STATE: u64 = 64 * 1024 * 1024;
pub const MAX_IMAGE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_IMAGE_PIXELS: u64 = 16_000_000;
fn validate_png(bytes: &[u8]) -> Result<(), StoreError> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(StoreError::ProjectLimit);
    }
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return Err(StoreError::InvalidProject);
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS {
        return Err(StoreError::ProjectLimit);
    }
    Ok(())
}
#[derive(Clone, Debug)]
pub struct Document {
    pub id: Uuid,
    pub name: String,
    pub path: PathBuf,
    /// Vault-relative path using `/`; the stable identity of the document.
    pub rel: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub rel: String,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
}
const MAX_DEPTH: usize = 12;
const MAX_NODES: usize = 5000;
const MAX_NAME: usize = 100;

/// Legacy `<UUID>.md` files at the root keep their UUID so existing state loads;
/// every other path gets a deterministic FNV-1a derived id.
fn id_for(rel: &str) -> Uuid {
    if let Some(id) = rel
        .strip_suffix(".md")
        .filter(|stem| !stem.contains('/'))
        .and_then(|stem| Uuid::parse_str(stem).ok())
    {
        return id;
    }
    let mut hash: u128 = 0x6c62272e07bb014262b821756295c58d;
    for byte in rel.bytes() {
        hash ^= u128::from(byte);
        hash = hash.wrapping_mul(0x0000000001000000000000000000013b);
    }
    Uuid::from_u128(hash | 1)
}
fn safe_rel(rel: &str) -> bool {
    !rel.is_empty()
        && rel.split('/').next() != Some("assets")
        && rel.split('/').all(|part| {
            !part.is_empty() && !part.starts_with('.') && !part.contains(['\\', ':', '\0'])
        })
}
fn valid_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && name.chars().count() <= MAX_NAME
        && name != "assets"
        && !name.starts_with('.')
        && !name.contains(['/', '\\', ':', '\0'])
}
pub struct Loaded {
    pub document: Document,
    pub workbench: Workbench,
    pub baseline: String,
    pub recovered_comments: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    version: u32,
    id: Uuid,
    workbench: WorkbenchSnapshot,
}
/// Sidebar/tab layout restored on launch; never contains document text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiState {
    pub expanded: Vec<String>,
    pub tabs: Vec<String>,
    pub active: Option<String>,
    pub right_tab: String,
}
#[derive(Clone)]
pub struct Library {
    root: PathBuf,
}
impl Library {
    pub fn from_env() -> Result<Self, StoreError> {
        let root = std::env::var_os("AGENT_DOCS_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".agent-docs"))
            })
            .ok_or(StoreError::InvalidPath)?;
        Self::open(root)
    }
    pub fn open(root: PathBuf) -> Result<Self, StoreError> {
        if !root.is_absolute() {
            return Err(StoreError::InvalidPath);
        }
        for path in [&root, &root.join(".state")] {
            if fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink()) {
                return Err(StoreError::UnsafeTarget);
            }
        }
        fs::create_dir_all(root.join(".state"))?;
        for path in [&root, &root.join(".state")] {
            if fs::symlink_metadata(path)?.file_type().is_symlink() {
                return Err(StoreError::UnsafeTarget);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
            }
        }
        Ok(Self { root })
    }
    pub fn load_ui(&self) -> UiState {
        let path = self.root.join(".state").join("ui.json");
        let mut bytes = Vec::new();
        let ok = fs::symlink_metadata(&path).is_ok_and(|m| m.is_file())
            && fs::File::open(&path)
                .and_then(|f| f.take(1 << 20).read_to_end(&mut bytes))
                .is_ok();
        let mut state: UiState = ok
            .then(|| serde_json::from_slice(&bytes).ok())
            .flatten()
            .unwrap_or_default();
        state.expanded.retain(|r| safe_rel(r));
        state.tabs.retain(|r| safe_rel(r) && r.ends_with(".md"));
        state.tabs.truncate(32);
        state.expanded.truncate(MAX_NODES);
        state
    }
    pub fn save_ui(&self, state: &UiState) -> Result<(), StoreError> {
        let dir = self.root.join(".state");
        let temp = dir.join(format!("{}.tmp", Uuid::new_v4()));
        fs::write(
            &temp,
            serde_json::to_vec(state).map_err(|_| StoreError::InvalidProject)?,
        )?;
        fs::rename(temp, dir.join("ui.json"))?;
        Ok(())
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    /// Immutable, vault-root-relative attachments survive document renames.
    pub fn store_image(&self, png: &[u8]) -> Result<String, StoreError> {
        validate_png(png)?;
        let dir = self.root.join("assets");
        if !dir.exists() {
            fs::create_dir(&dir)?;
        }
        if !fs::symlink_metadata(&dir)?.is_dir() {
            return Err(StoreError::UnsafeTarget);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
        }
        let rel = format!("assets/{}.png", Uuid::new_v4());
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let path = self.root.join(&rel);
        let result = (|| {
            let mut file = options.open(&path)?;
            file.write_all(png)?;
            file.sync_all()
        })();
        if let Err(e) = result {
            let _ = fs::remove_file(path);
            return Err(e.into());
        }
        Ok(rel)
    }
    pub fn read_image(&self, rel: &str) -> Result<Vec<u8>, StoreError> {
        let name = rel.strip_prefix("assets/").ok_or(StoreError::InvalidPath)?;
        let stem = name.strip_suffix(".png").ok_or(StoreError::InvalidPath)?;
        if Uuid::parse_str(stem).is_err() || name.contains('/') {
            return Err(StoreError::InvalidPath);
        }
        if !fs::symlink_metadata(self.root.join("assets"))?.is_dir()
            || !fs::symlink_metadata(self.root.join(rel))?.is_file()
        {
            return Err(StoreError::UnsafeTarget);
        }
        let mut bytes = Vec::new();
        fs::File::open(self.root.join(rel))?
            .take(MAX_IMAGE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        validate_png(&bytes)?;
        Ok(bytes)
    }
    fn validate_document(&self, document: &Document) -> Result<(), StoreError> {
        if !safe_rel(&document.rel)
            || !document.rel.ends_with(".md")
            || document.id != id_for(&document.rel)
            || document.path != self.root.join(&document.rel)
        {
            return Err(StoreError::InvalidPath);
        }
        let mut current = self.root.clone();
        let parts: Vec<_> = document.rel.split('/').collect();
        for part in &parts[..parts.len() - 1] {
            current.push(part);
            if !fs::symlink_metadata(&current)?.is_dir() {
                return Err(StoreError::UnsafeTarget);
            }
        }
        Ok(())
    }
    fn document_at(&self, rel: String, path: PathBuf, text: &str) -> Document {
        let legacy = !rel.contains('/') && Uuid::parse_str(&rel[..rel.len() - 3]).is_ok();
        let name = if legacy {
            title(text)
        } else {
            let file = rel.rsplit('/').next().unwrap_or(&rel);
            file[..file.len() - 3].to_owned()
        };
        Document {
            id: id_for(&rel),
            name,
            path,
            rel,
        }
    }
    fn walk(
        &self,
        dir: &Path,
        prefix: &str,
        depth: usize,
        out: &mut Vec<Node>,
        docs: &mut Vec<(std::time::SystemTime, Document)>,
    ) -> Result<(), StoreError> {
        let mut entries: Vec<_> = fs::read_dir(dir)?.filter_map(Result::ok).collect();
        entries.sort_by_key(|e| e.file_name());
        let mut folders = Vec::new();
        let mut files = Vec::new();
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(meta) = fs::symlink_metadata(entry.path()) else {
                continue;
            };
            if name.starts_with('.')
                || (prefix.is_empty() && name == "assets")
                || meta.file_type().is_symlink()
            {
                continue;
            }
            if meta.is_dir() {
                folders.push(name);
            } else if meta.is_file() && name.ends_with(".md") && name.len() > 3 {
                files.push((name, meta.modified().unwrap_or(std::time::UNIX_EPOCH)));
            }
        }
        for name in folders {
            if out.len() >= MAX_NODES {
                return Err(StoreError::ProjectLimit);
            }
            let rel = format!("{prefix}{name}");
            out.push(Node {
                rel: rel.clone(),
                name: name.clone(),
                is_dir: true,
                depth,
            });
            if depth + 1 < MAX_DEPTH {
                self.walk(&dir.join(&name), &format!("{rel}/"), depth + 1, out, docs)?;
            }
        }
        for (name, modified) in files {
            if out.len() >= MAX_NODES {
                return Err(StoreError::ProjectLimit);
            }
            let path = dir.join(&name);
            let legacy = prefix.is_empty() && Uuid::parse_str(&name[..name.len() - 3]).is_ok();
            let text = if legacy {
                crate::load(&path)?.ok_or(StoreError::InvalidProject)?
            } else {
                String::new()
            };
            let doc = self.document_at(format!("{prefix}{name}"), path, &text);
            out.push(Node {
                rel: doc.rel.clone(),
                name: doc.name.clone(),
                is_dir: false,
                depth,
            });
            docs.push((modified, doc));
        }
        Ok(())
    }
    /// Folders first, then files, depth-first; hidden entries and symlinks are skipped.
    pub fn tree(&self) -> Result<Vec<Node>, StoreError> {
        let (mut nodes, mut docs) = (Vec::new(), Vec::new());
        self.walk(&self.root, "", 0, &mut nodes, &mut docs)?;
        Ok(nodes)
    }
    pub fn document(&self, rel: &str) -> Result<Document, StoreError> {
        let document = Document {
            id: id_for(rel),
            name: String::new(),
            path: self.root.join(rel),
            rel: rel.to_owned(),
        };
        self.validate_document(&document)?;
        let text = crate::load(&document.path)?.ok_or(StoreError::InvalidProject)?;
        Ok(self.document_at(document.rel, document.path, &text))
    }
    pub fn list(&self) -> Result<Vec<Document>, StoreError> {
        let (mut nodes, mut docs) = (Vec::new(), Vec::new());
        self.walk(&self.root, "", 0, &mut nodes, &mut docs)?;
        docs.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.rel.cmp(&b.1.rel)));
        Ok(docs.into_iter().map(|(_, doc)| doc).collect())
    }
    fn dir_path(&self, rel: &str) -> Result<PathBuf, StoreError> {
        let mut current = self.root.clone();
        if rel.is_empty() {
            return Ok(current);
        }
        if !safe_rel(rel) {
            return Err(StoreError::InvalidPath);
        }
        for part in rel.split('/') {
            current.push(part);
            if !fs::symlink_metadata(&current)?.is_dir() {
                return Err(StoreError::UnsafeTarget);
            }
        }
        Ok(current)
    }
    fn unique(dir: &Path, name: &str, suffix: &str) -> PathBuf {
        let mut candidate = dir.join(format!("{name}{suffix}"));
        let mut n = 1;
        while fs::symlink_metadata(&candidate).is_ok() {
            candidate = dir.join(format!("{name} {n}{suffix}"));
            n += 1;
        }
        candidate
    }
    fn rel_of(&self, path: &Path) -> Result<String, StoreError> {
        Ok(path
            .strip_prefix(&self.root)
            .map_err(|_| StoreError::InvalidPath)?
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/"))
    }
    pub fn create_note(&self, dir: &str, name: &str) -> Result<Loaded, StoreError> {
        if !valid_name(name) {
            return Err(StoreError::InvalidPath);
        }
        let name = name.trim();
        let parent = self.dir_path(dir)?;
        let path = Self::unique(&parent, name, ".md");
        let rel = self.rel_of(&path)?;
        let text = format!(
            "# {}\n\n",
            &rel[rel.rfind('/').map_or(0, |i| i + 1)..rel.len() - 3]
        );
        let document = self.document_at(rel, path, &text);
        let workbench = Workbench::new(text.clone()).map_err(|_| StoreError::InvalidProject)?;
        self.save(&document, &workbench.snapshot(), None)?;
        Ok(Loaded {
            document,
            workbench,
            baseline: text,
            recovered_comments: true,
        })
    }
    pub fn create_folder(&self, parent: &str, name: &str) -> Result<String, StoreError> {
        if !valid_name(name) {
            return Err(StoreError::InvalidPath);
        }
        let path = Self::unique(&self.dir_path(parent)?, name.trim(), "");
        fs::create_dir(&path)?;
        self.rel_of(&path)
    }
    fn docs_under(&self, rel: &str) -> Result<Vec<String>, StoreError> {
        let (mut nodes, mut docs) = (Vec::new(), Vec::new());
        self.walk(
            &self.dir_path(rel)?,
            &format!("{rel}/"),
            0,
            &mut nodes,
            &mut docs,
        )?;
        Ok(docs.into_iter().map(|(_, d)| d.rel).collect())
    }
    fn state_path(&self, rel: &str) -> PathBuf {
        self.root
            .join(".state")
            .join(format!("{}.json", id_for(rel)))
    }
    fn migrate_state(&self, old: &str, new: &str) -> Result<(), StoreError> {
        let from = self.state_path(old);
        if !fs::symlink_metadata(&from).is_ok_and(|m| m.is_file()) {
            return Ok(());
        }
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&from)?).map_err(|_| StoreError::InvalidProject)?;
        value["id"] = serde_json::Value::String(id_for(new).to_string());
        let to = self.state_path(new);
        let temp = self
            .root
            .join(".state")
            .join(format!("{}.tmp", Uuid::new_v4()));
        fs::write(
            &temp,
            serde_json::to_vec(&value).map_err(|_| StoreError::InvalidProject)?,
        )?;
        fs::rename(temp, to)?;
        fs::remove_file(from)?;
        Ok(())
    }
    /// Renames a document (name without `.md`) or folder; comments follow the rename.
    pub fn rename(&self, rel: &str, new_name: &str) -> Result<String, StoreError> {
        if !valid_name(new_name) || !safe_rel(rel) {
            return Err(StoreError::InvalidPath);
        }
        let from = self.root.join(rel);
        let meta = fs::symlink_metadata(&from)?;
        if meta.file_type().is_symlink() {
            return Err(StoreError::UnsafeTarget);
        }
        let is_dir = meta.is_dir();
        let parent = rel.rfind('/').map_or("", |i| &rel[..i]);
        let leaf = format!("{}{}", new_name.trim(), if is_dir { "" } else { ".md" });
        let new_rel = if parent.is_empty() {
            leaf
        } else {
            format!("{parent}/{leaf}")
        };
        if new_rel == rel {
            return Ok(new_rel);
        }
        let to = self.root.join(&new_rel);
        if fs::symlink_metadata(&to).is_ok() {
            return Err(StoreError::ExternalChange);
        }
        let moved = if is_dir {
            self.docs_under(rel)?
        } else {
            vec![rel.to_owned()]
        };
        fs::rename(&from, &to)?;
        for old in moved {
            let new = format!("{new_rel}{}", &old[rel.len()..]);
            self.migrate_state(&old, &new)?;
        }
        Ok(new_rel)
    }
    /// Renames a readable-named document after its first `# ` title.
    /// Legacy UUID files, untitled text and already-matching names are left alone.
    pub fn sync_name(&self, rel: &str, text: &str) -> Result<Option<String>, StoreError> {
        let (dir, file) = rel.rsplit_once('/').unwrap_or(("", rel));
        let stem = file.strip_suffix(".md").unwrap_or(file);
        if dir.is_empty() && Uuid::parse_str(stem).is_ok() {
            return Ok(None);
        }
        let Some(heading) = text
            .lines()
            .find_map(|l| l.strip_prefix("# ").map(str::trim))
            .filter(|t| !t.is_empty())
        else {
            return Ok(None);
        };
        let wanted: String = heading
            .chars()
            .map(|c| if "/\\:\0".contains(c) { '-' } else { c })
            .take(MAX_NAME)
            .collect();
        let wanted = wanted.trim().trim_start_matches('.').trim().to_owned();
        if wanted.is_empty() || stem == wanted {
            return Ok(None);
        }
        let suffix = stem.strip_prefix(&wanted).and_then(|r| r.strip_prefix(' '));
        if suffix.is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) {
            return Ok(None);
        }
        let parent = self.dir_path(dir)?;
        let path = Self::unique(&parent, &wanted, ".md");
        let name = path
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or(StoreError::InvalidPath)?;
        self.rename(rel, &name).map(Some)
    }
    /// Deletes a document or a folder that only contains documents and folders.
    pub fn delete(&self, rel: &str) -> Result<(), StoreError> {
        if !safe_rel(rel) {
            return Err(StoreError::InvalidPath);
        }
        let path = self.root.join(rel);
        let meta = fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            return Err(StoreError::UnsafeTarget);
        }
        let docs = if meta.is_dir() {
            fn clean(dir: &Path) -> Result<(), StoreError> {
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let meta = fs::symlink_metadata(entry.path())?;
                    let ok = (meta.is_dir() && clean(&entry.path()).is_ok())
                        || (meta.is_file() && entry.file_name().to_string_lossy().ends_with(".md"));
                    if !ok {
                        return Err(StoreError::UnsafeTarget);
                    }
                }
                Ok(())
            }
            clean(&path)?;
            let docs = self.docs_under(rel)?;
            fs::remove_dir_all(&path)?;
            docs
        } else {
            fs::remove_file(&path)?;
            vec![rel.to_owned()]
        };
        for doc in docs {
            let _ = fs::remove_file(self.state_path(&doc));
        }
        Ok(())
    }
    pub fn create(&self) -> Result<Loaded, StoreError> {
        let id = Uuid::new_v4();
        let document = Document {
            id,
            name: "未命名文档".into(),
            path: self.root.join(format!("{id}.md")),
            rel: format!("{id}.md"),
        };
        let workbench =
            Workbench::new("# 未命名文档\n\n").map_err(|_| StoreError::InvalidProject)?;
        self.save(&document, &workbench.snapshot(), None)?;
        let baseline = workbench.text().to_owned();
        Ok(Loaded {
            document,
            workbench,
            baseline,
            recovered_comments: true,
        })
    }
    pub fn load(&self, document: &Document) -> Result<Loaded, StoreError> {
        self.validate_document(document)?;
        let text = crate::load(&document.path)?.ok_or(StoreError::InvalidProject)?;
        let metadata = self
            .root
            .join(".state")
            .join(format!("{}.json", document.id));
        let restored = (|| {
            if !fs::symlink_metadata(&metadata).ok()?.is_file()
                || fs::symlink_metadata(&metadata)
                    .ok()?
                    .file_type()
                    .is_symlink()
            {
                return None;
            }
            let mut bytes = Vec::new();
            fs::File::open(metadata)
                .ok()?
                .take(MAX_STATE + 1)
                .read_to_end(&mut bytes)
                .ok()?;
            if bytes.len() as u64 > MAX_STATE {
                return None;
            }
            let state: State = serde_json::from_slice(&bytes).ok()?;
            if state.version != 1 || state.id != document.id {
                return None;
            }
            let w = Workbench::restore(state.workbench).ok()?;
            (w.text() == text).then_some(w)
        })();
        let recovered_comments = restored.is_some();
        let workbench = restored
            .unwrap_or(Workbench::new(text.clone()).map_err(|_| StoreError::InvalidProject)?);
        Ok(Loaded {
            document: Document {
                name: title(&text),
                ..document.clone()
            },
            workbench,
            baseline: text,
            recovered_comments,
        })
    }
    /// Markdown first, metadata second. A crash between them never lets stale
    /// comment anchors overwrite canonical text; mismatched metadata is ignored.
    pub fn save(
        &self,
        document: &Document,
        snapshot: &WorkbenchSnapshot,
        expected: Option<&str>,
    ) -> Result<(), StoreError> {
        self.validate_document(document)?;
        let workbench =
            Workbench::restore(snapshot.clone()).map_err(|_| StoreError::InvalidProject)?;
        let bytes = serde_json::to_vec(&State {
            version: 1,
            id: document.id,
            workbench: snapshot.clone(),
        })
        .map_err(|_| StoreError::InvalidProject)?;
        if bytes.len() as u64 > MAX_STATE {
            return Err(StoreError::ProjectLimit);
        }
        // A prior metadata failure may already have committed this Markdown.
        // Permit an idempotent retry only when disk equals the intended text.
        let current = crate::load(&document.path)?;
        if current.as_deref() != expected && current.as_deref() != Some(workbench.text()) {
            return Err(StoreError::ExternalChange);
        }
        crate::save(&document.path, workbench.text(), current.as_deref())?;
        let state_path = self
            .root
            .join(".state")
            .join(format!("{}.json", document.id));
        if fs::symlink_metadata(&state_path)
            .is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
        {
            return Err(StoreError::UnsafeTarget);
        }
        let temp = self
            .root
            .join(".state")
            .join(format!("{}.tmp", Uuid::new_v4()));
        let mut options = fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp)?;
        let _cleanup = super::RemoveOnDrop(temp.clone());
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(temp, state_path)?;
        fs::File::open(self.root.join(".state"))?.sync_all()?;
        Ok(())
    }
}
pub fn title(text: &str) -> String {
    text.lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|s| !s.is_empty())
        .unwrap_or("未命名文档")
        .chars()
        .take(48)
        .collect()
}
