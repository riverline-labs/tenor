use std::path::Path;
use std::process;

pub(crate) fn cmd_test(suite_dir: &Path, quiet: bool) {
    if !suite_dir.exists() {
        eprintln!(
            "error: conformance suite directory not found: {}",
            suite_dir.display()
        );
        process::exit(1);
    }

    let _ = quiet; // TAP output is the primary output; quiet has no effect on test runner
    let result = crate::runner::run_suite(suite_dir);
    if result.failed > 0 {
        process::exit(1);
    }
}
