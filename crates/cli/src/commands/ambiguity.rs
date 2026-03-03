use std::path::Path;
use std::process;

pub(crate) fn cmd_ambiguity(suite_dir: &Path, spec: Option<&Path>, model: Option<&str>) {
    let spec_path = spec.map(|p| p.to_path_buf()).unwrap_or_else(|| {
        suite_dir
            .parent()
            .unwrap_or(std::path::Path::new(".."))
            .join("docs/tenor-language-specification.md")
    });

    if !suite_dir.exists() {
        eprintln!(
            "error: conformance suite directory not found: {}",
            suite_dir.display()
        );
        process::exit(1);
    }

    let result = crate::ambiguity::run_ambiguity_suite(suite_dir, &spec_path, model);
    eprintln!(
        "\nAmbiguity test summary: {} total, {} matches, {} mismatches, {} hard errors",
        result.total, result.matches, result.mismatches, result.hard_errors
    );
    if result.hard_errors > 0 {
        process::exit(1);
    }
}
