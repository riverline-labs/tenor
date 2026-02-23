//! Source provider abstraction for filesystem-independent elaboration.
//!
//! The [`SourceProvider`] trait abstracts file I/O so the elaborator can work
//! without `std::fs` -- a prerequisite for WASM compilation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Trait that abstracts file I/O for the elaboration pipeline.
///
/// Implementations provide source text reading, import resolution, and path
/// canonicalization. The default [`FileSystemProvider`] delegates to `std::fs`;
/// [`InMemoryProvider`] enables WASM and testing without filesystem access.
pub trait SourceProvider {
    /// Read the source text for a given path. Returns the content as a String.
    fn read_source(&self, path: &Path) -> Result<String, std::io::Error>;

    /// Resolve a relative import path against a base directory.
    fn resolve_import(&self, base: &Path, import: &str) -> Result<PathBuf, std::io::Error>;

    /// Canonicalize a path for cycle detection and sandbox checks.
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, std::io::Error>;
}

/// Default filesystem-backed source provider.
///
/// Delegates to `std::fs::read_to_string`, `Path::join`, and
/// `Path::canonicalize`. This is what the existing `elaborate(&Path)`
/// function uses internally.
pub struct FileSystemProvider;

impl SourceProvider for FileSystemProvider {
    fn read_source(&self, path: &Path) -> Result<String, std::io::Error> {
        std::fs::read_to_string(path)
    }

    fn resolve_import(&self, base: &Path, import: &str) -> Result<PathBuf, std::io::Error> {
        Ok(base.join(import))
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, std::io::Error> {
        path.canonicalize()
    }
}

/// In-memory source provider for WASM and testing.
///
/// Maps paths to source text strings. Canonicalization normalizes the path
/// without requiring filesystem access.
pub struct InMemoryProvider {
    files: HashMap<PathBuf, String>,
}

impl InMemoryProvider {
    /// Create a new in-memory provider from a map of paths to source text.
    pub fn new(files: HashMap<PathBuf, String>) -> Self {
        Self { files }
    }

    /// Normalize a path by resolving `.` and `..` components without
    /// touching the filesystem.
    fn normalize_path(path: &Path) -> PathBuf {
        let mut components = Vec::new();
        for component in path.components() {
            match component {
                std::path::Component::CurDir => {} // skip "."
                std::path::Component::ParentDir => {
                    // pop unless we are at root
                    if !components.is_empty() {
                        components.pop();
                    }
                }
                other => components.push(other),
            }
        }
        components.iter().collect()
    }
}

impl SourceProvider for InMemoryProvider {
    fn read_source(&self, path: &Path) -> Result<String, std::io::Error> {
        let normalized = Self::normalize_path(path);
        self.files.get(&normalized).cloned().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("file not found in memory: {}", normalized.display()),
            )
        })
    }

    fn resolve_import(&self, base: &Path, import: &str) -> Result<PathBuf, std::io::Error> {
        Ok(Self::normalize_path(&base.join(import)))
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, std::io::Error> {
        let normalized = Self::normalize_path(path);
        if self.files.contains_key(&normalized) {
            Ok(normalized)
        } else {
            let is_prefix = self.files.keys().any(|k| k.starts_with(&normalized));
            if is_prefix {
                Ok(normalized)
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!(
                        "path not found in memory provider: {}",
                        normalized.display()
                    ),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_resolves_dot_and_dotdot() {
        let p = Path::new("/a/b/../c/./d");
        let normalized = InMemoryProvider::normalize_path(p);
        assert_eq!(normalized, PathBuf::from("/a/c/d"));
    }

    #[test]
    fn in_memory_read_source_found() {
        let mut files = HashMap::new();
        files.insert(PathBuf::from("/test.tenor"), "fact Foo {}".to_string());
        let provider = InMemoryProvider::new(files);
        let content = provider.read_source(Path::new("/test.tenor")).unwrap();
        assert_eq!(content, "fact Foo {}");
    }

    #[test]
    fn in_memory_read_source_not_found() {
        let provider = InMemoryProvider::new(HashMap::new());
        let err = provider
            .read_source(Path::new("/missing.tenor"))
            .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn in_memory_resolve_import() {
        let mut files = HashMap::new();
        files.insert(PathBuf::from("/dir/sub.tenor"), "fact X {}".to_string());
        let provider = InMemoryProvider::new(files);
        let resolved = provider
            .resolve_import(Path::new("/dir"), "sub.tenor")
            .unwrap();
        assert_eq!(resolved, PathBuf::from("/dir/sub.tenor"));
    }

    #[test]
    fn in_memory_canonicalize_existing_file() {
        let mut files = HashMap::new();
        files.insert(PathBuf::from("/a/b/test.tenor"), "fact X {}".to_string());
        let provider = InMemoryProvider::new(files);
        let canon = provider
            .canonicalize(Path::new("/a/b/../b/test.tenor"))
            .unwrap();
        assert_eq!(canon, PathBuf::from("/a/b/test.tenor"));
    }

    #[test]
    fn in_memory_canonicalize_directory_prefix() {
        let mut files = HashMap::new();
        files.insert(PathBuf::from("/root/test.tenor"), "fact X {}".to_string());
        let provider = InMemoryProvider::new(files);
        let canon = provider.canonicalize(Path::new("/root")).unwrap();
        assert_eq!(canon, PathBuf::from("/root"));
    }

    #[test]
    fn in_memory_canonicalize_missing_returns_error() {
        let provider = InMemoryProvider::new(HashMap::new());
        let err = provider
            .canonicalize(Path::new("/nonexistent"))
            .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
