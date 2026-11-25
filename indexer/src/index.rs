use std::collections::HashMap;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use anyhow::{Context, Result, ensure};
use ignore::WalkBuilder;
use rusqlite::{Connection, OpenFlags, Transaction, TransactionBehavior, params};

use crate::Match;
use crate::extract::Language;

// Bump when the schema or extraction rules change to invalidate cached results.
const INDEX_VERSION: u32 = 2;

pub fn update(root: &Path) -> Result<()> {
    let path = database_path(root)?;
    fs::create_dir_all(path.parent().expect("database path has a parent"))?;
    let mut connection = Connection::open(&path)
        .with_context(|| format!("opening {}", path.display()))?;
    connection.busy_timeout(Duration::from_secs(10))?;
    connection.pragma_update(None, "foreign_keys", true)?;

    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    initialize(&transaction)?;
    refresh(&transaction, root)?;
    transaction.commit()?;
    Ok(())
}

pub fn query(root: &Path, identifier: &str, limit: u32) -> Result<Vec<Match>> {
    read_index(root, |db| lookup(db, identifier, limit))
}

pub fn features(root: &Path) -> Result<Vec<String>> {
    read_index(root, |db| {
        let mut statement = db.prepare("SELECT DISTINCT feature FROM annotations ORDER BY feature")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    })
}

fn read_index<T>(root: &Path, read: impl FnOnce(&Transaction<'_>) -> Result<T>) -> Result<T> {
    let path = database_path(root)?;
    let mut connection = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| {
            format!(
                "could not open index at {}; run `contextor index` to create it",
                path.display()
            )
        })?;
    connection.busy_timeout(Duration::from_secs(10))?;

    let transaction = connection.transaction()?;
    let version: u32 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    ensure!(
        version == INDEX_VERSION,
        "incompatible index at {}; run `contextor index` to rebuild it",
        path.display()
    );
    let result = read(&transaction)?;
    transaction.commit()?;
    Ok(result)
}

fn database_path(root: &Path) -> Result<PathBuf> {
    let base = dirs::home_dir().context("could not determine the home directory")?;
    let slug = root.to_string_lossy().split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .to_ascii_lowercase();
    let slug = if slug.is_empty() { "root" } else { &slug };
    let suffix = &slug[slug.len().saturating_sub(80)..];
    let hash = blake3::hash(root.as_os_str().as_encoded_bytes()).to_hex();
    Ok(base.join(".contextor")
        .join(format!("{suffix}-{}", &hash[..16]))
        .join("index.sqlite"))
}

fn initialize(db: &Transaction<'_>) -> Result<()> {
    let version: u32 = db.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version == INDEX_VERSION {
        return Ok(());
    }
    db.execute_batch(
        "DROP TABLE IF EXISTS annotations;
         DROP TABLE IF EXISTS files;
         CREATE TABLE files (
             path TEXT PRIMARY KEY NOT NULL,
             stamp TEXT,
             hash BLOB NOT NULL
         );
         CREATE TABLE annotations (
             path TEXT NOT NULL REFERENCES files(path) ON DELETE CASCADE,
             feature TEXT NOT NULL,
             symbol_name TEXT,
             start_line INTEGER,
             end_line INTEGER
         );
         CREATE INDEX annotations_path ON annotations (path);
         CREATE INDEX annotations_lookup ON annotations (feature, path, start_line, end_line, symbol_name);",
    )?;
    db.pragma_update(None, "user_version", INDEX_VERSION)?;
    Ok(())
}

struct CachedFile {
    stamp: Option<String>,
    hash: Vec<u8>,
}

fn inventory(db: &Transaction<'_>) -> Result<HashMap<String, CachedFile>> {
    let mut statement = db.prepare("SELECT path, stamp, hash FROM files")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get(0)?, CachedFile { stamp: row.get(1)?, hash: row.get(2)? }))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn refresh(db: &Transaction<'_>, root: &Path) -> Result<()> {
    let mut unseen = inventory(db)?;
    let walker = WalkBuilder::new(root)
        .require_git(false)
        .filter_entry(|entry| entry.depth() == 0 || !matches!(
            entry.file_name().to_str(), Some("node_modules" | ".git")
        ))
        .build();

    for entry in walker {
        let entry = entry.context("walking project files")?;
        let path = entry.path();
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let Some(language) = Language::for_path(path) else { continue };
        let Some(relative) = path.strip_prefix(root)?.to_str() else { continue };
        let previous = unseen.remove(relative);
        refresh_file(db, path, relative, language, previous.as_ref())
            .with_context(|| format!("indexing {}", path.display()))?;
    }
    let mut delete = db.prepare("DELETE FROM files WHERE path = ?1")?;
    for path in unseen.keys() {
        delete.execute([path])?;
    }
    Ok(())
}

fn file_stamp(metadata: &Metadata) -> Option<String> {
    let modified = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    let stamp = format!("{}:{}", metadata.len(), modified.as_nanos());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Some(format!("{stamp}:{}:{}:{}:{}", metadata.dev(), metadata.ino(),
            metadata.ctime(), metadata.ctime_nsec()))
    }
    #[cfg(not(unix))]
    Some(stamp)
}

fn refresh_file(
    db: &Transaction<'_>,
    path: &Path,
    relative: &str,
    language: Language,
    previous: Option<&CachedFile>,
) -> Result<()> {
    let stamp = file_stamp(&path.metadata()?);
    if stamp.is_some() && previous.is_some_and(|file| file.stamp == stamp) {
        return Ok(());
    }
    let bytes = fs::read(path)?;
    let hash = blake3::hash(&bytes);
    db.prepare_cached(
        "INSERT INTO files (path, stamp, hash) VALUES (?1, ?2, ?3)
         ON CONFLICT(path) DO UPDATE SET stamp = excluded.stamp, hash = excluded.hash",
    )?.execute(params![relative, stamp, hash.as_bytes().as_slice()])?;
    if previous.is_some_and(|file| file.hash.as_slice() == hash.as_bytes()) {
        return Ok(());
    }

    db.prepare_cached("DELETE FROM annotations WHERE path = ?1")?.execute([relative])?;
    if let Ok(source) = std::str::from_utf8(&bytes) {
        let annotations = language.extract(source)?;
        let mut insert = db.prepare_cached(
            "INSERT INTO annotations (path, feature, symbol_name, start_line, end_line)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for annotation in annotations {
            let (start, end) = annotation.lines.unzip();
            insert.execute(params![relative, annotation.feature, annotation.symbol, start, end])?;
        }
    }
    Ok(())
}

fn lookup(db: &Transaction<'_>, identifier: &str, limit: u32) -> Result<Vec<Match>> {
    let mut statement = db.prepare(
        "SELECT path, symbol_name, start_line, end_line FROM annotations
         WHERE feature = ?1 ORDER BY path, start_line, end_line, symbol_name LIMIT ?2",
    )?;
    let rows = statement.query_map(params![identifier, limit], |row| {
        Ok(Match {
            path: row.get(0)?,
            symbol_name: row.get(1)?,
            start_line: row.get(2)?,
            end_line: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}
