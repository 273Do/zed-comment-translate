//! Translations kept in memory and in a file, so that documentation seen
//! before is shown without a request: instantly, offline and after a restart.
//!
//! Entries are keyed by the source text, so the same documentation hovered in
//! different places (e.g. `String`) shares one entry. The file holds one JSON
//! array `["source", "translation" | null]` per line and is only appended to.
//! With `persist_cache: false` there is no file and entries last until exit.

use std::{
    collections::HashMap,
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Default)]
pub struct Cache {
    /// `None` means the text is already in the target language.
    entries: HashMap<String, Option<String>>,
    file: Option<PathBuf>,
}

impl Cache {
    /// Loads `<dir>/<name>.jsonl`. Without a directory, entries are only kept in memory.
    pub fn open(dir: Option<PathBuf>, name: &str) -> Self {
        let Some(dir) = dir else {
            return Self::default();
        };
        let name: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let file = dir.join(format!("{name}.jsonl"));
        let entries = fs::read_to_string(&file)
            .map(|content| parse(&content))
            .unwrap_or_default();
        log!(
            "loaded {} cached translations from {}",
            entries.len(),
            file.display()
        );
        Self {
            entries,
            file: Some(file),
        }
    }

    pub fn get(&self, text: &str) -> Option<&Option<String>> {
        self.entries.get(text)
    }

    pub fn contains(&self, text: &str) -> bool {
        self.entries.contains_key(text)
    }

    pub fn extend(&mut self, entries: impl IntoIterator<Item = (String, Option<String>)>) {
        let mut lines = String::new();
        for (text, translation) in entries {
            lines += &serde_json::json!([text, translation]).to_string();
            lines.push('\n');
            self.entries.insert(text, translation);
        }
        if let Some(file) = &self.file {
            if let Err(error) = append(file, &lines) {
                log!("failed to write {}: {error}", file.display());
            }
        }
    }
}

/// `$COMMENT_TRANSLATE_CACHE_DIR`, or `comment-translate` in the platform's cache directory.
pub fn default_dir() -> Option<PathBuf> {
    if let Some(dir) = env::var_os("COMMENT_TRANSLATE_CACHE_DIR") {
        return Some(dir.into());
    }
    let var = |name| env::var_os(name).map(PathBuf::from);
    let base = if cfg!(target_os = "macos") {
        var("HOME").map(|home| home.join("Library/Caches"))
    } else if cfg!(windows) {
        var("LOCALAPPDATA")
    } else {
        var("XDG_CACHE_HOME").or_else(|| var("HOME").map(|home| home.join(".cache")))
    };
    Some(base?.join("comment-translate"))
}

/// Deletes the cache files in `dir` (only `*.jsonl`, in case `dir` is shared).
pub fn clear(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path
            .extension()
            .is_some_and(|extension| extension == "jsonl")
        {
            match fs::remove_file(&path) {
                Ok(()) => log!("deleted {}", path.display()),
                Err(error) => log!("failed to delete {}: {error}", path.display()),
            }
        }
    }
}

/// Later lines win; broken lines (e.g. a write cut short) are skipped.
fn parse(content: &str) -> HashMap<String, Option<String>> {
    content
        .lines()
        .filter_map(|line| {
            let entry: (String, Option<String>) = serde_json::from_str(line).ok()?;
            Some(entry)
        })
        .collect()
}

fn append(file: &Path, lines: &str) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)?
        .write_all(lines.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_entries() {
        let dir = env::temp_dir().join(format!("comment-translate-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);

        let mut cache = Cache::open(Some(dir.clone()), "google-auto-ja");
        cache.extend([
            (
                "A String.\nLine two.".to_string(),
                Some("文字列。".to_string()),
            ),
            ("既に日本語".to_string(), None),
        ]);
        cache.extend([(
            "A String.\nLine two.".to_string(),
            Some("文字列です。".to_string()),
        )]);

        let reopened = Cache::open(Some(dir.clone()), "google-auto-ja");
        assert_eq!(
            reopened.get("A String.\nLine two."),
            Some(&Some("文字列です。".to_string()))
        );
        assert_eq!(reopened.get("既に日本語"), Some(&None));
        assert!(!reopened.contains("missing"));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn clears_only_cache_files() {
        let dir = env::temp_dir().join(format!("comment-translate-clear-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        Cache::open(Some(dir.clone()), "google-auto-ja").extend([("a".to_string(), None)]);
        fs::write(dir.join("other.txt"), "").unwrap();

        clear(&dir);
        assert!(!dir.join("google-auto-ja.jsonl").exists());
        assert!(dir.join("other.txt").exists());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn skips_broken_lines() {
        let entries = parse("[\"a\",\"あ\"]\n[\"b\",\n[\"c\",null]\n");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries["c"], None);
    }
}
