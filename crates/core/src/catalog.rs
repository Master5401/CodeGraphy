use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{cmp::Ordering, collections::BTreeMap};

use anyhow::{Context, Result, anyhow, bail};
use ignore::WalkBuilder;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

const MAX_INDEXED_TEXT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_HASHED_FILE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_SOURCE_PREVIEW_BYTES: usize = 1024 * 1024;
const INDEX_BATCH_SIZE: usize = 64;

#[derive(Clone, Debug)]
pub struct WorkspaceStore {
    root: PathBuf,
    database_path: PathBuf,
    workspace_id: String,
    repository_id: String,
}

#[derive(Debug, Clone, Copy)]
pub struct AnalysisFactQuery<'a> {
    pub provider: &'a str,
    pub profile: &'a str,
    pub fact_kind: &'a str,
    pub relation_kind: Option<&'a str>,
    pub resolution_state: Option<&'a str>,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct FileRecord {
    pub id: String,
    pub workspace_id: String,
    pub repository_id: String,
    pub path: String,
    pub display_name: String,
    pub extension: Option<String>,
    pub language: Option<String>,
    pub detected_format: String,
    pub byte_size: u64,
    pub content_hash: Option<String>,
    pub is_binary: bool,
    pub capability_level: u8,
    pub parse_status: String,
    pub classification_method: String,
    pub diagnostic: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SearchHit {
    pub file: FileRecord,
    pub snippet: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkspaceEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub file: Option<FileRecord>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DirectoryListing {
    pub path: String,
    pub entries: Vec<WorkspaceEntry>,
    pub has_more: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourcePreview {
    pub path: String,
    pub content: String,
    pub truncated: bool,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct CatalogProgress {
    pub files_seen: u64,
    pub files_indexed: u64,
    pub text_indexed: u64,
    pub diagnostics: u64,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct CatalogReport {
    pub files_seen: u64,
    pub files_indexed: u64,
    pub text_indexed: u64,
    pub diagnostics: u64,
    pub stale_files_removed: u64,
}

#[derive(Clone, Debug)]
struct PendingFile {
    record: FileRecord,
    search_text: String,
}

impl WorkspaceStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = fs::canonicalize(root.as_ref()).with_context(|| {
            format!("cannot resolve workspace path {}", root.as_ref().display())
        })?;
        if !root.is_dir() {
            bail!("workspace path is not a directory: {}", root.display());
        }

        let workspace_id = stable_id("workspace", &[&canonical_path_text(&root)]);
        let repository_id = stable_id("repository", &[&workspace_id]);
        let state_dir = root.join(".codewiki");
        fs::create_dir_all(&state_dir)
            .with_context(|| format!("cannot create state directory {}", state_dir.display()))?;
        let database_path = state_dir.join("codewiki.db");
        let store = Self {
            root,
            database_path,
            workspace_id,
            repository_id,
        };
        store.initialize()?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub fn index_workspace<F>(&self, mut report_progress: F) -> Result<CatalogReport>
    where
        F: FnMut(CatalogProgress),
    {
        let mut connection = self.connect()?;
        connection.execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS scan_seen(file_id TEXT PRIMARY KEY);
             DELETE FROM scan_seen;",
        )?;

        let mut progress = CatalogProgress::default();
        let mut pending = Vec::with_capacity(INDEX_BATCH_SIZE);
        let mut walker = WalkBuilder::new(&self.root);
        walker
            .hidden(false)
            .follow_links(false)
            .git_ignore(true)
            .git_exclude(true)
            .ignore(true)
            .parents(true)
            .require_git(false)
            .filter_entry(|entry| {
                entry
                    .path()
                    .file_name()
                    .is_none_or(|name| name != ".codewiki")
            })
            .sort_by_file_path(|left, right| left.cmp(right));

        for result in walker.build() {
            let entry = match result {
                Ok(entry) => entry,
                Err(_) => {
                    progress.diagnostics += 1;
                    report_progress(progress);
                    continue;
                }
            };
            if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                continue;
            }

            progress.files_seen += 1;
            match self.inspect_file(entry.path()) {
                Ok(file) => {
                    if file.record.capability_level >= 1 {
                        progress.text_indexed += 1;
                    }
                    if file.record.diagnostic.is_some() {
                        progress.diagnostics += 1;
                    }
                    pending.push(file);
                    if pending.len() >= INDEX_BATCH_SIZE {
                        self.write_batch(&mut connection, &pending)?;
                        progress.files_indexed += pending.len() as u64;
                        pending.clear();
                    }
                }
                Err(error) => {
                    progress.diagnostics += 1;
                    pending.push(self.unreadable_file(entry.path(), &format!("{error:#}"))?);
                    if pending.len() >= INDEX_BATCH_SIZE {
                        self.write_batch(&mut connection, &pending)?;
                        progress.files_indexed += pending.len() as u64;
                        pending.clear();
                    }
                }
            }
            report_progress(progress);
        }

        if !pending.is_empty() {
            self.write_batch(&mut connection, &pending)?;
            progress.files_indexed += pending.len() as u64;
        }

        let stale_files_removed = if progress.diagnostics == 0 {
            let removed = connection.execute(
                "DELETE FROM files WHERE file_id NOT IN (SELECT file_id FROM scan_seen)",
                [],
            )?;
            removed as u64
        } else {
            0
        };
        connection.execute_batch("DROP TABLE scan_seen;")?;
        connection.execute_batch("PRAGMA optimize;")?;

        progress.complete = true;
        report_progress(progress);
        Ok(CatalogReport {
            files_seen: progress.files_seen,
            files_indexed: progress.files_indexed,
            text_indexed: progress.text_indexed,
            diagnostics: progress.diagnostics,
            stale_files_removed,
        })
    }

    pub fn list_files(&self, limit: u32, offset: u32) -> Result<Vec<SearchHit>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT file_id, workspace_id, repository_id, path, display_name, extension,
                    language, detected_format,
                    byte_size, content_hash, is_binary, capability_level, parse_status,
                    classification_method, diagnostic, NULL
             FROM files
             ORDER BY path COLLATE NOCASE, path
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows = statement.query_map(params![limit.min(100), offset], map_search_hit)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("could not read file catalog")
    }

    pub fn search_files(&self, query: &str, limit: u32, offset: u32) -> Result<Vec<SearchHit>> {
        let query = fts_query(query);
        if query.is_empty() {
            return self.list_files(limit, offset);
        }
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT f.file_id, f.workspace_id, f.repository_id, f.path, f.display_name,
                    f.extension, f.language, f.detected_format, f.byte_size,
                    f.content_hash, f.is_binary, f.capability_level, f.parse_status,
                    f.classification_method, f.diagnostic, snippet(files_fts, 2, '[', ']', '…', 18)
             FROM files_fts
             JOIN files AS f ON f.rowid = files_fts.rowid
             WHERE files_fts MATCH ?1
             ORDER BY bm25(files_fts), f.path COLLATE NOCASE, f.path
             LIMIT ?2 OFFSET ?3",
        )?;
        let rows = statement.query_map(params![query, limit.min(100), offset], map_search_hit)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("could not search indexed text")
    }

    pub fn list_directory(
        &self,
        relative_directory: &str,
        limit: u32,
        offset: u32,
    ) -> Result<DirectoryListing> {
        let directory_path = normalize_directory_argument(relative_directory)?;
        let connection = self.connect()?;
        let (sql, prefix) = if directory_path.is_empty() {
            (
                "SELECT file_id, workspace_id, repository_id, path, display_name,
                        extension, language, detected_format, byte_size, content_hash,
                        is_binary, capability_level, parse_status, classification_method,
                        diagnostic, NULL
                 FROM files ORDER BY path COLLATE NOCASE, path",
                String::new(),
            )
        } else {
            (
                "SELECT file_id, workspace_id, repository_id, path, display_name,
                        extension, language, detected_format, byte_size, content_hash,
                        is_binary, capability_level, parse_status, classification_method,
                        diagnostic, NULL
                 FROM files
                 WHERE substr(path, 1, length(?1) + 1) = ?1 || '/'
                 ORDER BY path COLLATE NOCASE, path",
                directory_path.clone(),
            )
        };
        let mut statement = connection.prepare(sql)?;
        let rows = if prefix.is_empty() {
            statement.query_map([], map_search_hit)?
        } else {
            statement.query_map([prefix], map_search_hit)?
        };
        let mut children = BTreeMap::<String, WorkspaceEntry>::new();
        for row in rows {
            let file = row?.file;
            let remainder = if directory_path.is_empty() {
                file.path.as_str()
            } else {
                file.path
                    .strip_prefix(&format!("{directory_path}/"))
                    .context("catalog path is outside requested directory")?
            };
            let (name, child_path, is_directory, child_file) =
                if let Some((name, _)) = remainder.split_once('/') {
                    let child_path = if directory_path.is_empty() {
                        name.to_owned()
                    } else {
                        format!("{directory_path}/{name}")
                    };
                    (name.to_owned(), child_path, true, None)
                } else {
                    (
                        file.display_name.clone(),
                        file.path.clone(),
                        false,
                        Some(file),
                    )
                };
            children
                .entry(child_path.clone())
                .or_insert(WorkspaceEntry {
                    name,
                    path: child_path,
                    is_directory,
                    file: child_file,
                });
        }

        let mut entries = children.into_values().collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            match (left.is_directory, right.is_directory) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => Ordering::Equal,
            }
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.name.cmp(&right.name))
        });
        let offset = offset as usize;
        let limit = limit.clamp(1, 500) as usize;
        let has_more = entries.len() > offset.saturating_add(limit);
        let entries = entries.into_iter().skip(offset).take(limit).collect();
        Ok(DirectoryListing {
            path: directory_path,
            entries,
            has_more,
        })
    }

    pub fn read_source_preview(&self, file_id: &str) -> Result<SourcePreview> {
        let connection = self.connect()?;
        let path: Option<String> = connection
            .query_row(
                "SELECT path FROM files WHERE file_id = ?1",
                [file_id],
                |row| row.get(0),
            )
            .optional()?;
        let relative_path = path.ok_or_else(|| anyhow!("file is not in the workspace catalog"))?;
        let candidate = self.root.join(Path::new(&relative_path));
        let resolved_path = fs::canonicalize(&candidate)
            .with_context(|| format!("file is unavailable: {relative_path}"))?;
        if !resolved_path.starts_with(&self.root) {
            bail!("file resolves outside the workspace");
        }
        let metadata = fs::metadata(&resolved_path)?;
        if !metadata.is_file() {
            bail!("catalog entry is not a regular file");
        }

        let file = File::open(&resolved_path)?;
        let mut bytes = Vec::with_capacity(MAX_SOURCE_PREVIEW_BYTES + 1);
        file.take((MAX_SOURCE_PREVIEW_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        let truncated = bytes.len() > MAX_SOURCE_PREVIEW_BYTES;
        if truncated {
            bytes.truncate(MAX_SOURCE_PREVIEW_BYTES);
        }
        if bytes.contains(&0) {
            bail!("binary files do not have a text preview");
        }
        let content = String::from_utf8(bytes).context("source preview is not UTF-8 text")?;
        Ok(SourcePreview {
            path: relative_path,
            content,
            truncated,
        })
    }

    pub fn file_count(&self) -> Result<u64> {
        let connection = self.connect()?;
        let count: i64 =
            connection.query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    /// Persist a versioned provider snapshot and its individual queryable facts.
    pub fn save_analysis_report(&self, report: &serde_json::Value) -> Result<()> {
        let string_field = |name: &str| -> Result<&str> {
            report
                .get(name)
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow!("analysis report is missing string field `{name}`"))
        };
        let provider = string_field("provider")?;
        let provider_version = string_field("provider_version")?;
        let profile = string_field("profile")?;
        let schema_version = string_field("schema")?;
        let summary = report
            .get("summary")
            .ok_or_else(|| anyhow!("analysis report is missing summary"))?;
        let summary_json = serde_json::to_string(summary)?;
        let report_json = serde_json::to_string(report)?;

        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO analysis_runs(provider, profile, schema_version, provider_version, summary_json, report_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(provider, profile) DO UPDATE SET
                 schema_version = excluded.schema_version,
                 provider_version = excluded.provider_version,
                 summary_json = excluded.summary_json,
                 report_json = excluded.report_json",
            params![provider, profile, schema_version, provider_version, summary_json, report_json],
        )?;
        transaction.execute(
            "DELETE FROM analysis_facts WHERE provider = ?1 AND profile = ?2",
            params![provider, profile],
        )?;

        for fact_kind in [
            "artifacts",
            "symbols",
            "relations",
            "unresolved_calls",
            "diagnostics",
        ] {
            let Some(facts) = report.get(fact_kind).and_then(serde_json::Value::as_array) else {
                continue;
            };
            for (index, fact) in facts.iter().enumerate() {
                let payload = serde_json::to_string(fact)?;
                let fact_id = fact
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .unwrap_or_else(|| stable_id("analysis-fact", &[fact_kind, &payload]));
                let path = fact
                    .get("evidence")
                    .and_then(|evidence| evidence.get("path"))
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| fact.get("path").and_then(serde_json::Value::as_str));
                let resolution_state = fact
                    .get("resolution_state")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| {
                        fact.get("evidence")
                            .and_then(|evidence| evidence.get("resolution_state"))
                            .and_then(serde_json::Value::as_str)
                    });
                let relation_kind = (fact_kind == "relations")
                    .then(|| fact.get("kind").and_then(serde_json::Value::as_str))
                    .flatten();
                transaction.execute(
                    "INSERT INTO analysis_facts(provider, profile, fact_id, fact_kind, relation_kind, resolution_state, path, payload_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        provider,
                        profile,
                        fact_id,
                        fact_kind.trim_end_matches('s'),
                        relation_kind,
                        resolution_state,
                        path,
                        payload,
                    ],
                ).with_context(|| format!("could not persist {fact_kind} fact #{index}"))?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn analysis_run_summary(
        &self,
        provider: &str,
        profile: &str,
    ) -> Result<Option<serde_json::Value>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT schema_version, provider_version, summary_json
             FROM analysis_runs WHERE provider = ?1 AND profile = ?2",
        )?;
        let result = statement
            .query_row(params![provider, profile], |row| {
                let schema: String = row.get(0)?;
                let provider_version: String = row.get(1)?;
                let summary_json: String = row.get(2)?;
                Ok((schema, provider_version, summary_json))
            })
            .optional()?;
        result
            .map(|(schema, provider_version, summary)| {
                Ok(serde_json::json!({
                    "schema": schema,
                    "provider": provider,
                    "provider_version": provider_version,
                    "profile": profile,
                    "summary": serde_json::from_str::<serde_json::Value>(&summary)?,
                }))
            })
            .transpose()
    }

    pub fn query_analysis_facts(
        &self,
        query: AnalysisFactQuery<'_>,
    ) -> Result<Vec<serde_json::Value>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT payload_json FROM analysis_facts
             WHERE provider = ?1 AND profile = ?2 AND fact_kind = ?3
               AND (?4 IS NULL OR relation_kind = ?4)
               AND (?5 IS NULL OR resolution_state = ?5)
             ORDER BY path COLLATE NOCASE, path, fact_id
             LIMIT ?6 OFFSET ?7",
        )?;
        let rows = statement.query_map(
            params![
                query.provider,
                query.profile,
                query.fact_kind,
                query.relation_kind,
                query.resolution_state,
                query.limit.clamp(1, 100) as i64,
                query.offset,
            ],
            |row| {
                let payload: String = row.get(0)?;
                serde_json::from_str(&payload).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })
            },
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("could not read persisted analysis facts")
    }

    fn initialize(&self) -> Result<()> {
        let connection = self.connect()?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS workspaces (
                 workspace_id TEXT PRIMARY KEY,
                 root_path TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS files (
                 file_id TEXT PRIMARY KEY,
                 workspace_id TEXT NOT NULL,
                 repository_id TEXT NOT NULL,
                 path TEXT NOT NULL UNIQUE,
                 display_name TEXT NOT NULL,
                 extension TEXT,
                 language TEXT,
                 detected_format TEXT NOT NULL,
                 byte_size INTEGER NOT NULL,
                 content_hash TEXT,
                 is_binary INTEGER NOT NULL,
                 capability_level INTEGER NOT NULL,
                 parse_status TEXT NOT NULL,
                 classification_method TEXT NOT NULL,
                 diagnostic TEXT,
                 search_text TEXT NOT NULL DEFAULT ''
             );
             CREATE TABLE IF NOT EXISTS analysis_runs (
                 provider TEXT NOT NULL,
                 profile TEXT NOT NULL,
                 schema_version TEXT NOT NULL,
                 provider_version TEXT NOT NULL,
                 summary_json TEXT NOT NULL,
                 report_json TEXT NOT NULL,
                 PRIMARY KEY(provider, profile)
             );
             CREATE TABLE IF NOT EXISTS analysis_facts (
                 provider TEXT NOT NULL,
                 profile TEXT NOT NULL,
                 fact_id TEXT NOT NULL,
                 fact_kind TEXT NOT NULL,
                 relation_kind TEXT,
                 resolution_state TEXT,
                 path TEXT,
                 payload_json TEXT NOT NULL,
                 PRIMARY KEY(provider, profile, fact_id),
                 FOREIGN KEY(provider, profile) REFERENCES analysis_runs(provider, profile) ON DELETE CASCADE
             );
             CREATE INDEX IF NOT EXISTS analysis_facts_query_idx
                 ON analysis_facts(provider, profile, fact_kind, relation_kind, resolution_state, path);
             CREATE INDEX IF NOT EXISTS files_path_idx ON files(path);
             CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
                 path, display_name, search_text,
                 content='files', content_rowid='rowid'
             );
             CREATE TRIGGER IF NOT EXISTS files_ai AFTER INSERT ON files BEGIN
                 INSERT INTO files_fts(rowid, path, display_name, search_text)
                 VALUES (new.rowid, new.path, new.display_name, new.search_text);
             END;
             CREATE TRIGGER IF NOT EXISTS files_ad AFTER DELETE ON files BEGIN
                 INSERT INTO files_fts(files_fts, rowid, path, display_name, search_text)
                 VALUES ('delete', old.rowid, old.path, old.display_name, old.search_text);
             END;
             CREATE TRIGGER IF NOT EXISTS files_au AFTER UPDATE ON files BEGIN
                 INSERT INTO files_fts(files_fts, rowid, path, display_name, search_text)
                 VALUES ('delete', old.rowid, old.path, old.display_name, old.search_text);
                 INSERT INTO files_fts(rowid, path, display_name, search_text)
                 VALUES (new.rowid, new.path, new.display_name, new.search_text);
             END;
             PRAGMA user_version = 2;",
        )?;
        connection.execute(
            "INSERT INTO workspaces(workspace_id, root_path) VALUES (?1, ?2)
             ON CONFLICT(workspace_id) DO NOTHING",
            params![self.workspace_id, self.root.to_string_lossy()],
        )?;
        Ok(())
    }

    fn connect(&self) -> Result<Connection> {
        let connection = Connection::open(&self.database_path)
            .with_context(|| format!("cannot open index {}", self.database_path.display()))?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok(connection)
    }

    fn inspect_file(&self, path: &Path) -> Result<PendingFile> {
        let relative = path
            .strip_prefix(&self.root)
            .context("walked file is outside the workspace root")?;
        let relative_path = normalize_relative_path(relative);
        let display_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| relative_path.clone());
        let extension = path
            .extension()
            .map(|value| value.to_string_lossy().to_ascii_lowercase());
        let id = stable_id("file", &[&self.workspace_id, &relative_path]);
        let metadata = fs::metadata(path)?;
        let byte_size = metadata.len();
        let mut diagnostic = None;
        let mut content_hash = None;
        let mut search_text = String::new();
        let mut is_binary = false;
        let mut classification_method = "extension-hint".to_owned();
        let mut detected_format = format_from_extension(extension.as_deref()).to_owned();
        let mut language = language_from_extension(extension.as_deref()).map(str::to_owned);
        let parse_status;
        let mut capability_level = 0_u8;

        let prefix = read_prefix(path, 8 * 1024)?;
        if let Some(magic_format) = format_from_magic(&prefix) {
            detected_format = magic_format.to_owned();
            classification_method = "magic-bytes".to_owned();
        }

        if byte_size <= MAX_INDEXED_TEXT_BYTES {
            let bytes = fs::read(path)?;
            content_hash = Some(blake3::hash(&bytes).to_hex().to_string());
            if let Some(format) = format_from_magic(&bytes) {
                detected_format = format.to_owned();
                classification_method = "magic-bytes".to_owned();
            }
            match String::from_utf8(bytes) {
                Ok(text) if !text.contains('\0') && !is_binary_format(&detected_format) => {
                    if language.is_none() && detected_format == "unknown" {
                        detected_format = "text".to_owned();
                        classification_method = "utf8-text".to_owned();
                    }
                    search_text = text;
                    capability_level = 1;
                    parse_status = "text-indexed".to_owned();
                }
                _ => {
                    is_binary = true;
                    parse_status = "binary".to_owned();
                }
            }
        } else if byte_size <= MAX_HASHED_FILE_BYTES {
            content_hash = Some(hash_file(path)?);
            if !is_binary_format(&detected_format) {
                diagnostic = Some(format!(
                    "text indexing skipped above the {} byte limit",
                    MAX_INDEXED_TEXT_BYTES
                ));
                parse_status = "text-limit-exceeded".to_owned();
            } else {
                is_binary = true;
                parse_status = "binary".to_owned();
            }
        } else {
            is_binary = true;
            diagnostic = Some(format!(
                "content hash skipped above the {} byte safety limit",
                MAX_HASHED_FILE_BYTES
            ));
            parse_status = "size-limit-exceeded".to_owned();
        }

        if is_binary && language.is_some() {
            language = None;
        }

        Ok(PendingFile {
            record: FileRecord {
                id,
                workspace_id: self.workspace_id.clone(),
                repository_id: self.repository_id.clone(),
                path: relative_path,
                display_name,
                extension,
                language,
                detected_format,
                byte_size,
                content_hash,
                is_binary,
                capability_level,
                parse_status,
                classification_method,
                diagnostic,
            },
            search_text,
        })
    }

    fn unreadable_file(&self, path: &Path, reason: &str) -> Result<PendingFile> {
        let relative = path
            .strip_prefix(&self.root)
            .context("walked file is outside the workspace root")?;
        let relative_path = normalize_relative_path(relative);
        let display_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| relative_path.clone());
        let extension = path
            .extension()
            .map(|value| value.to_string_lossy().to_ascii_lowercase());
        let id = stable_id("file", &[&self.workspace_id, &relative_path]);
        Ok(PendingFile {
            record: FileRecord {
                id,
                workspace_id: self.workspace_id.clone(),
                repository_id: self.repository_id.clone(),
                path: relative_path,
                display_name,
                extension: extension.clone(),
                language: None,
                detected_format: format_from_extension(extension.as_deref()).to_owned(),
                byte_size: 0,
                content_hash: None,
                is_binary: false,
                capability_level: 0,
                parse_status: "unreadable".to_owned(),
                classification_method: "extension-hint".to_owned(),
                diagnostic: Some(reason.to_owned()),
            },
            search_text: String::new(),
        })
    }

    fn write_batch(&self, connection: &mut Connection, files: &[PendingFile]) -> Result<()> {
        let transaction = connection.transaction()?;
        for file in files {
            let record = &file.record;
            transaction.execute(
                "INSERT INTO files(
                     file_id, workspace_id, repository_id, path, display_name, extension,
                     language, detected_format, byte_size, content_hash, is_binary,
                     capability_level, parse_status, classification_method, diagnostic, search_text
                 ) VALUES (
                     ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16
                 ) ON CONFLICT(file_id) DO UPDATE SET
                     path = excluded.path,
                     display_name = excluded.display_name,
                     extension = excluded.extension,
                     language = excluded.language,
                     detected_format = excluded.detected_format,
                     byte_size = excluded.byte_size,
                     content_hash = excluded.content_hash,
                     is_binary = excluded.is_binary,
                     capability_level = excluded.capability_level,
                     parse_status = excluded.parse_status,
                     classification_method = excluded.classification_method,
                     diagnostic = excluded.diagnostic,
                     search_text = excluded.search_text",
                params![
                    record.id,
                    self.workspace_id,
                    self.repository_id,
                    record.path,
                    record.display_name,
                    record.extension,
                    record.language,
                    record.detected_format,
                    record.byte_size as i64,
                    record.content_hash,
                    record.is_binary,
                    record.capability_level,
                    record.parse_status,
                    record.classification_method,
                    record.diagnostic,
                    file.search_text,
                ],
            )?;
            transaction.execute(
                "INSERT OR IGNORE INTO scan_seen(file_id) VALUES (?1)",
                [&record.id],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }
}

fn map_search_hit(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchHit> {
    Ok(SearchHit {
        file: FileRecord {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            repository_id: row.get(2)?,
            path: row.get(3)?,
            display_name: row.get(4)?,
            extension: row.get(5)?,
            language: row.get(6)?,
            detected_format: row.get(7)?,
            byte_size: row.get::<_, i64>(8)? as u64,
            content_hash: row.get(9)?,
            is_binary: row.get(10)?,
            capability_level: row.get(11)?,
            parse_status: row.get(12)?,
            classification_method: row.get(13)?,
            diagnostic: row.get(14)?,
        },
        snippet: row.get(15)?,
    })
}

fn fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter(|term| term.chars().any(char::is_alphanumeric))
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn stable_id(kind: &str, parts: &[&str]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(kind.as_bytes());
    for part in parts {
        hasher.update(&(part.len() as u64).to_le_bytes());
        hasher.update(part.as_bytes());
    }
    format!("{}:{}", kind, hasher.finalize().to_hex())
}

fn canonical_path_text(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        path.to_ascii_lowercase()
    } else {
        path
    }
}

fn normalize_relative_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn normalize_directory_argument(path: &str) -> Result<String> {
    let parsed = Path::new(path);
    if parsed.is_absolute() {
        bail!("directory path must be relative to the workspace");
    }
    for component in parsed.components() {
        if !matches!(
            component,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        ) {
            bail!("directory path contains an unsupported component");
        }
    }
    Ok(normalize_relative_path(parsed))
}

fn read_prefix(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut bytes = vec![0; limit];
    let read = file.read(&mut bytes)?;
    bytes.truncate(read);
    Ok(bytes)
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn format_from_extension(extension: Option<&str>) -> &'static str {
    match extension.unwrap_or_default() {
        "c" | "h" => "c_source",
        "cc" | "cpp" | "cxx" | "hh" | "hpp" | "hxx" => "cpp_source",
        "cs" => "csharp_source",
        "go" => "go_source",
        "rs" => "rust_source",
        "py" | "pyi" => "python_source",
        "js" | "mjs" | "cjs" => "javascript_source",
        "jsx" => "javascript_react_source",
        "ts" | "mts" | "cts" => "typescript_source",
        "tsx" => "typescript_react_source",
        "java" => "java_source",
        "kt" | "kts" => "kotlin_source",
        "php" => "php_source",
        "rb" => "ruby_source",
        "swift" => "swift_source",
        "scala" | "sc" => "scala_source",
        "xml" => "xml",
        "yaml" | "yml" => "yaml",
        "json" | "jsonc" => "json",
        "toml" => "toml",
        "ini" | "properties" | "env" => "configuration_text",
        "sql" => "sql",
        "md" | "markdown" => "markdown",
        "rst" => "restructuredtext",
        "adoc" | "asciidoc" => "asciidoc",
        "txt" => "plain_text",
        "html" | "htm" => "html",
        "csv" => "csv",
        "tsv" => "tsv",
        "jsonl" | "ndjson" => "jsonl",
        "class" => "java_class",
        "jar" | "war" | "zip" => "archive",
        "pdf" => "pdf",
        "png" => "png_image",
        "jpg" | "jpeg" => "jpeg_image",
        "gif" => "gif_image",
        "woff" | "woff2" | "ttf" | "otf" => "font",
        "mp3" | "wav" | "ogg" | "flac" => "audio",
        "mp4" | "mov" | "webm" | "mkv" => "video",
        _ => "unknown",
    }
}

fn language_from_extension(extension: Option<&str>) -> Option<&'static str> {
    match extension.unwrap_or_default() {
        "c" | "h" => Some("c"),
        "cc" | "cpp" | "cxx" | "hh" | "hpp" | "hxx" => Some("cpp"),
        "cs" => Some("csharp"),
        "go" => Some("go"),
        "rs" => Some("rust"),
        "py" | "pyi" => Some("python"),
        "js" | "mjs" | "cjs" | "jsx" => Some("javascript"),
        "ts" | "mts" | "cts" | "tsx" => Some("typescript"),
        "java" => Some("java"),
        "kt" | "kts" => Some("kotlin"),
        "php" => Some("php"),
        "rb" => Some("ruby"),
        "swift" => Some("swift"),
        "scala" | "sc" => Some("scala"),
        _ => None,
    }
}

fn format_from_magic(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\xCA\xFE\xBA\xBE") {
        Some("java_class")
    } else if bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06") {
        Some("archive")
    } else if bytes.starts_with(b"%PDF-") {
        Some("pdf")
    } else if bytes.starts_with(b"\x7FELF") {
        Some("elf_binary")
    } else if bytes.starts_with(b"MZ") {
        Some("pe_binary")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1A\n") {
        Some("png_image")
    } else if bytes.starts_with(b"\xFF\xD8\xFF") {
        Some("jpeg_image")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif_image")
    } else if bytes.starts_with(b"wOFF") || bytes.starts_with(b"wOF2") || bytes.starts_with(b"OTTO")
    {
        Some("font")
    } else if bytes.starts_with(b"ID3") || bytes.starts_with(b"RIFF") {
        Some("media_binary")
    } else {
        None
    }
}

fn is_binary_format(format: &str) -> bool {
    matches!(
        format,
        "java_class"
            | "archive"
            | "pdf"
            | "elf_binary"
            | "pe_binary"
            | "png_image"
            | "jpeg_image"
            | "gif_image"
            | "font"
            | "audio"
            | "video"
            | "media_binary"
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::{AnalysisFactQuery, WorkspaceStore};

    fn fixture_root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("codewiki-catalog-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture root");
        root
    }

    #[test]
    fn catalogs_polyglot_files_and_searches_readable_text() {
        let root = fixture_root("polyglot");
        fs::create_dir_all(root.join("src")).expect("create source dir");
        fs::write(
            root.join("src/Hello.java"),
            "class Hello { void greet() {} }\n",
        )
        .expect("write Java source");
        fs::write(root.join("config.yaml"), "service:\n  timeout: 30\n").expect("write config");
        fs::write(
            root.join("notes.md"),
            "# Workspace\nSearchable words here.\n",
        )
        .expect("write documentation");
        fs::write(root.join("opaque.bin"), [0, 1, 2, 255]).expect("write binary fixture");
        fs::write(root.join(".gitignore"), "ignored.txt\n").expect("write ignore file");
        fs::write(root.join("ignored.txt"), "should not appear").expect("write ignored text");

        let store = WorkspaceStore::open(&root).expect("open index");
        let report = store.index_workspace(|_| {}).expect("index fixture");
        assert_eq!(report.files_indexed, 5);
        assert_eq!(report.text_indexed, 4);

        let all_files = store.list_files(100, 0).expect("list files");
        assert_eq!(all_files.len(), 5);
        assert!(
            all_files
                .iter()
                .any(|hit| hit.file.language.as_deref() == Some("java"))
        );
        assert!(
            all_files
                .iter()
                .any(|hit| hit.file.detected_format == "yaml")
        );
        assert!(all_files.iter().any(|hit| hit.file.is_binary));
        assert!(!all_files.iter().any(|hit| hit.file.path == "ignored.txt"));

        let hits = store
            .search_files("timeout 30", 20, 0)
            .expect("search text");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].file.path, "config.yaml");
        assert!(
            hits[0]
                .snippet
                .as_deref()
                .is_some_and(|snippet| snippet.contains("timeout"))
        );
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[test]
    fn refresh_removes_deleted_files_and_source_preview_is_rooted() {
        let root = fixture_root("refresh");
        fs::write(root.join("readme.txt"), "source preview line\n").expect("write source");
        let store = WorkspaceStore::open(&root).expect("open index");
        store.index_workspace(|_| {}).expect("initial index");

        let hit = store
            .list_files(10, 0)
            .expect("list")
            .into_iter()
            .next()
            .expect("one file");
        let preview = store
            .read_source_preview(&hit.file.id)
            .expect("read source preview");
        assert!(preview.content.contains("source preview line"));
        assert!(store.read_source_preview("../outside").is_err());

        fs::remove_file(root.join("readme.txt")).expect("delete source");
        let report = store.index_workspace(|_| {}).expect("refresh index");
        assert_eq!(report.stale_files_removed, 1);
        assert!(
            store
                .list_files(10, 0)
                .expect("list after refresh")
                .is_empty()
        );
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[test]
    fn directory_listing_mirrors_workspace_paths_and_rejects_escape_paths() {
        let root = fixture_root("tree");
        fs::create_dir_all(root.join("src/nested")).expect("create nested source dirs");
        fs::write(root.join("root.txt"), "root text").expect("write root file");
        fs::write(root.join("src/A.java"), "class A {}\n").expect("write Java file");
        fs::write(root.join("src/nested/B.java"), "class B {}\n").expect("write nested Java file");

        let store = WorkspaceStore::open(&root).expect("open index");
        store.index_workspace(|_| {}).expect("index fixture");

        let root_listing = store.list_directory("", 200, 0).expect("list root");
        assert_eq!(root_listing.entries.len(), 2);
        assert!(root_listing.entries[0].is_directory);
        assert_eq!(root_listing.entries[0].path, "src");
        assert_eq!(root_listing.entries[1].path, "root.txt");
        let first_page = store
            .list_directory("", 1, 0)
            .expect("list first root page");
        assert!(first_page.has_more);
        assert_eq!(first_page.entries[0].path, "src");
        let second_page = store
            .list_directory("", 1, 1)
            .expect("list second root page");
        assert!(!second_page.has_more);
        assert_eq!(second_page.entries[0].path, "root.txt");

        let nested_listing = store.list_directory("src", 200, 0).expect("list src");
        assert_eq!(nested_listing.entries.len(), 2);
        assert!(
            nested_listing
                .entries
                .iter()
                .any(|entry| entry.path == "src/nested")
        );
        assert!(
            nested_listing
                .entries
                .iter()
                .any(|entry| entry.path == "src/A.java")
        );
        assert!(store.list_directory("../outside", 200, 0).is_err());
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[test]
    fn provider_analysis_is_persisted_and_queryable_with_resolution() {
        let root = fixture_root("analysis");
        let store = WorkspaceStore::open(&root).expect("open index");
        let report = serde_json::json!({
            "schema": "codegraphy-analysis/v1",
            "provider": "atlas-engine",
            "provider_version": "1.7.0",
            "profile": "java-structural",
            "summary": { "indexed_files": 1, "symbols": 2, "edges": 1 },
            "artifacts": [],
            "symbols": [],
            "relations": [{
                "id": "relation:stable",
                "kind": "calls",
                "source_name": "proof.Source.start",
                "target_name": "proof.Target.finish",
                "resolution_state": "HEURISTIC",
                "evidence": { "path": "src/Source.java", "line_start": 8 }
            }],
            "unresolved_calls": [{
                "id": "call:missing",
                "name": "unknown",
                "resolution_state": "UNRESOLVED",
                "evidence": { "path": "src/Source.java", "line_start": 12 }
            }],
            "diagnostics": []
        });
        store
            .save_analysis_report(&report)
            .expect("persist Atlas report");

        let summary = store
            .analysis_run_summary("atlas-engine", "java-structural")
            .expect("read analysis summary")
            .expect("Atlas run exists");
        assert_eq!(summary["provider_version"], "1.7.0");
        let calls = store
            .query_analysis_facts(AnalysisFactQuery {
                provider: "atlas-engine",
                profile: "java-structural",
                fact_kind: "relation",
                relation_kind: Some("calls"),
                resolution_state: Some("HEURISTIC"),
                limit: 100,
                offset: 0,
            })
            .expect("query heuristic call edges");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["source_name"], "proof.Source.start");
        let unresolved = store
            .query_analysis_facts(AnalysisFactQuery {
                provider: "atlas-engine",
                profile: "java-structural",
                fact_kind: "unresolved_call",
                relation_kind: None,
                resolution_state: None,
                limit: 100,
                offset: 0,
            })
            .expect("query unresolved callsites");
        assert_eq!(unresolved.len(), 1);
        assert_eq!(unresolved[0]["resolution_state"], "UNRESOLVED");
        fs::remove_dir_all(root).expect("remove fixture root");
    }
}
