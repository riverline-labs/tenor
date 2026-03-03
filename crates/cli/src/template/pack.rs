//! Template archive creation and extraction (`tenor pack`).

use std::path::{Path, PathBuf};

use super::manifest::{self, TemplateManifest, TemplateManifestFile};

/// Result returned by a successful `pack_template` invocation.
pub struct PackResult {
    pub archive_path: PathBuf,
    pub archive_hash: String,
    pub manifest: TemplateManifestFile,
}

/// Pack a template directory into a `.tenor-template.tar.gz` archive.
///
/// # Arguments
///
/// * `template_dir` — directory containing `tenor-template.toml` and `contract/`
/// * `output` — optional path for the archive; defaults to
///   `{name}-{version}.tenor-template.tar.gz` in the current directory
pub fn pack_template(template_dir: &Path, output: Option<&Path>) -> Result<PackResult, String> {
    // 1. Read and parse the manifest
    let manifest_file = manifest::read_manifest(template_dir)?;
    let tmpl: &TemplateManifest = &manifest_file.template;

    // 2. Validate the manifest
    tmpl.validate()?;

    // 3. Find .tenor files in contract/
    let contract_dir = template_dir.join("contract");
    if !contract_dir.exists() {
        return Err(format!(
            "contract/ directory not found in '{}'",
            template_dir.display()
        ));
    }

    let tenor_files = collect_files(&contract_dir, "tenor")?;
    if tenor_files.is_empty() {
        return Err(format!(
            "no .tenor files found in '{}'",
            contract_dir.display()
        ));
    }

    // 4. Elaborate the contract — find the main file
    let main_tenor = find_main_tenor(&tenor_files, tmpl);
    let bundle_json = elaborate_contract(main_tenor)?;

    // 5. Build the archive in a temp location, then move to final path
    let archive_filename = manifest::archive_filename(tmpl);
    let final_path = match output {
        Some(p) => p.to_path_buf(),
        None => PathBuf::from(&archive_filename),
    };

    let tmp_dir = tempfile::tempdir().map_err(|e| format!("could not create temp dir: {}", e))?;
    let tmp_archive = tmp_dir.path().join(&archive_filename);

    create_archive(template_dir, &bundle_json, &tmp_archive)?;

    // 6. Compute SHA-256 hash of the archive
    let archive_bytes =
        std::fs::read(&tmp_archive).map_err(|e| format!("could not read archive: {}", e))?;
    let archive_hash = sha256_hex(&archive_bytes);

    // 7. Move to final destination
    std::fs::copy(&tmp_archive, &final_path).map_err(|e| {
        format!(
            "could not write archive to '{}': {}",
            final_path.display(),
            e
        )
    })?;

    Ok(PackResult {
        archive_path: final_path,
        archive_hash,
        manifest: manifest_file,
    })
}

/// Unpack a `.tenor-template.tar.gz` archive into `output_dir`.
///
/// Returns the parsed manifest from the unpacked directory.
// Used by `tenor install` (Phase 11 Plan 03) — not yet wired to a CLI command.
#[allow(dead_code)]
pub fn unpack_template(archive: &Path, output_dir: &Path) -> Result<TemplateManifestFile, String> {
    let archive_file =
        std::fs::File::open(archive).map_err(|e| format!("could not open archive: {}", e))?;

    let gz = flate2::read::GzDecoder::new(archive_file);
    let mut tar = tar::Archive::new(gz);

    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("could not create output dir: {}", e))?;

    tar.unpack(output_dir)
        .map_err(|e| format!("could not unpack archive: {}", e))?;

    manifest::read_manifest(output_dir)
}

// ─── Internal helpers ────────────────────────────────────────────────────────

/// Collect all files with the given extension (no dot prefix) under `dir`.
fn collect_files(dir: &Path, extension: &str) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    let read_dir = std::fs::read_dir(dir)
        .map_err(|e| format!("could not read directory '{}': {}", dir.display(), e))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| format!("directory entry error: {}", e))?;
        let path = entry.path();
        if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == extension {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Select the main .tenor file to elaborate.
///
/// Prefers a file whose stem matches the template name; falls back to the first
/// file in sorted order.
fn find_main_tenor<'a>(tenor_files: &'a [PathBuf], tmpl: &TemplateManifest) -> &'a Path {
    for path in tenor_files {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            // Accept exact match or hyphen-to-underscore variation
            if stem == tmpl.name || stem == tmpl.name.replace('-', "_") {
                return path;
            }
        }
    }
    &tenor_files[0]
}

/// Elaborate a .tenor file and return its interchange JSON as a pretty-printed string.
fn elaborate_contract(path: &Path) -> Result<String, String> {
    let bundle = tenor_core::elaborate::elaborate(path)
        .map_err(|e| format!("contract does not elaborate: {:?}", e))?;

    serde_json::to_string_pretty(&bundle).map_err(|e| format!("could not serialize bundle: {}", e))
}

/// Create a `.tar.gz` archive at `archive_path` containing the template contents.
///
/// Archive structure (all paths relative to archive root):
/// - `tenor-template.toml`
/// - `contract/<files>`
/// - `bundle.json`
/// - `examples/` (if present)
/// - `screenshots/` (if present)
/// - `README.md` (if present)
fn create_archive(
    template_dir: &Path,
    bundle_json: &str,
    archive_path: &Path,
) -> Result<(), String> {
    use flate2::write::GzEncoder;
    use flate2::Compression;

    let archive_file = std::fs::File::create(archive_path)
        .map_err(|e| format!("could not create archive: {}", e))?;

    let gz = GzEncoder::new(archive_file, Compression::default());
    let mut tar = tar::Builder::new(gz);

    // tenor-template.toml
    add_file(
        &mut tar,
        template_dir,
        Path::new("tenor-template.toml"),
        "tenor-template.toml",
    )?;

    // contract/ directory
    let contract_dir = template_dir.join("contract");
    add_dir(&mut tar, &contract_dir, "contract")?;

    // bundle.json (the elaborated interchange)
    let bundle_bytes = bundle_json.as_bytes();
    let mut header = tar::Header::new_gnu();
    header.set_size(bundle_bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, "bundle.json", bundle_bytes)
        .map_err(|e| format!("could not add bundle.json to archive: {}", e))?;

    // Optional: examples/
    let examples_dir = template_dir.join("examples");
    if examples_dir.exists() {
        add_dir(&mut tar, &examples_dir, "examples")?;
    }

    // Optional: screenshots/
    let screenshots_dir = template_dir.join("screenshots");
    if screenshots_dir.exists() {
        add_dir(&mut tar, &screenshots_dir, "screenshots")?;
    }

    // Optional: README.md
    let readme = template_dir.join("README.md");
    if readme.exists() {
        add_file(&mut tar, template_dir, Path::new("README.md"), "README.md")?;
    }

    // Finalise
    tar.into_inner()
        .map_err(|e| format!("could not finalise archive gz: {}", e))?
        .finish()
        .map_err(|e| format!("could not finalise archive: {}", e))?;

    Ok(())
}

/// Add a single file to the archive under the given archive path.
fn add_file<W: std::io::Write>(
    tar: &mut tar::Builder<W>,
    base_dir: &Path,
    relative: &Path,
    archive_name: &str,
) -> Result<(), String> {
    let full_path = base_dir.join(relative);
    let mut file = std::fs::File::open(&full_path)
        .map_err(|e| format!("could not open '{}': {}", full_path.display(), e))?;

    let metadata = file
        .metadata()
        .map_err(|e| format!("could not stat '{}': {}", full_path.display(), e))?;

    let mut header = tar::Header::new_gnu();
    header.set_metadata(&metadata);
    header
        .set_path(archive_name)
        .map_err(|e| format!("could not set archive path: {}", e))?;
    header.set_cksum();

    tar.append(&header, &mut file)
        .map_err(|e| format!("could not add '{}' to archive: {}", archive_name, e))
}

/// Recursively add a directory to the archive with the given prefix.
fn add_dir<W: std::io::Write>(
    tar: &mut tar::Builder<W>,
    dir: &Path,
    archive_prefix: &str,
) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }

    let read_dir =
        std::fs::read_dir(dir).map_err(|e| format!("could not read '{}': {}", dir.display(), e))?;

    let mut entries: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();

    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let archive_path = format!("{}/{}", archive_prefix, name.to_string_lossy());

        if path.is_dir() {
            add_dir(tar, &path, &archive_path)?;
        } else if path.is_file() {
            let mut file = std::fs::File::open(&path)
                .map_err(|e| format!("could not open '{}': {}", path.display(), e))?;
            let metadata = file
                .metadata()
                .map_err(|e| format!("could not stat '{}': {}", path.display(), e))?;

            let mut header = tar::Header::new_gnu();
            header.set_metadata(&metadata);
            header
                .set_path(&archive_path)
                .map_err(|e| format!("could not set archive path: {}", e))?;
            header.set_cksum();

            tar.append(&header, &mut file)
                .map_err(|e| format!("could not add '{}' to archive: {}", archive_path, e))?;
        }
    }

    Ok(())
}

/// Compute SHA-256 of bytes and return lowercase hex string.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    let digest = sha2::Sha256::digest(bytes);
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}
