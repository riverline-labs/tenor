/// Conformance suite runner.
///
/// Convention:
///   positive/               — *.tenor + *.expected.json (no error expected)
///   negative/pass0..pass6/  — *.tenor + *.expected-error.json (error expected)
///   cross_file/             — rules.tenor (root) + facts.tenor (leaf) + bundle.expected.json
///   parallel/               — *.tenor + *.expected-error.json (pass 5 parallel entity conflict)
use crate::elaborate;
use crate::tap::Tap;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct RunResult {
    pub failed: usize,
}

pub fn run_suite(suite_dir: &Path) -> RunResult {
    let mut tap = Tap::new();

    // Positive tests
    run_positive_dir(suite_dir, "positive", &mut tap);

    // Negative tests by pass
    for pass in 0..=6 {
        run_negative_tests(suite_dir, pass, &mut tap);
    }

    // Cross-file tests
    run_cross_file_tests(suite_dir, &mut tap);

    // Parallel entity conflict tests
    run_parallel_tests(suite_dir, &mut tap);

    // Numeric precision tests
    run_positive_dir(suite_dir, "numeric", &mut tap);

    // Type promotion tests
    run_positive_dir(suite_dir, "promotion", &mut tap);

    // DSL shorthand expansion tests
    run_positive_dir(suite_dir, "shorthand", &mut tap);

    let failed = tap.failure_count();
    tap.finish();

    RunResult { failed }
}

fn run_positive_dir(suite_dir: &Path, subdir: &str, tap: &mut Tap) {
    let dir = suite_dir.join(subdir);
    if !dir.exists() {
        return;
    }
    let mut entries = glob_tenor_files(&dir);
    entries.sort();
    for tenor_path in &entries {
        let stem = stem(tenor_path);
        let expected_path = dir.join(format!("{}.expected.json", stem));
        if !expected_path.exists() {
            tap.not_ok(
                format!("{}/{}", subdir, stem),
                format!("missing expected file: {}", expected_path.display()),
            );
            continue;
        }
        run_positive_test(tenor_path, &expected_path, &stem, subdir, tap);
    }
}

fn run_negative_tests(suite_dir: &Path, pass: u8, tap: &mut Tap) {
    let dir = suite_dir.join(format!("negative/pass{}", pass));
    if !dir.exists() {
        return;
    }
    let mut entries = glob_tenor_files(&dir);
    entries.sort();

    // Multi-file tests: identified by *_a.tenor pattern — use the _a file as root.
    // Single-file tests: all other .tenor files.
    let mut roots: Vec<PathBuf> = Vec::new();
    for p in &entries {
        let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
        // Skip _b.tenor (leaf files in multi-file tests)
        if name.ends_with("_b.tenor") {
            continue;
        }
        roots.push(p.clone());
    }

    for tenor_path in &roots {
        let stem = stem(tenor_path);
        let expected_path = dir.join(format!("{}.expected-error.json", stem));
        if !expected_path.exists() {
            tap.not_ok(
                format!("negative/pass{}/{}", pass, stem),
                format!("missing expected-error file: {}", expected_path.display()),
            );
            continue;
        }
        run_negative_test(tenor_path, &expected_path, &stem, pass, tap);
    }
}

fn run_cross_file_tests(suite_dir: &Path, tap: &mut Tap) {
    let dir = suite_dir.join("cross_file");
    if !dir.exists() {
        return;
    }
    // The root file is the one with an import declaration.
    // Per convention: rules.tenor imports facts.tenor; expected output: bundle.expected.json
    let root = dir.join("rules.tenor");
    let expected = dir.join("bundle.expected.json");
    if root.exists() && expected.exists() {
        run_positive_test(&root, &expected, "bundle", "cross_file", tap);
    }
}

fn run_parallel_tests(suite_dir: &Path, tap: &mut Tap) {
    let dir = suite_dir.join("parallel");
    if !dir.exists() {
        return;
    }
    let mut entries = glob_tenor_files(&dir);
    entries.sort();
    for tenor_path in &entries {
        let stem = stem(tenor_path);
        let expected_path = dir.join(format!("{}.expected-error.json", stem));
        if !expected_path.exists() {
            continue; // parallel positive tests have no expected-error
        }
        run_negative_test(tenor_path, &expected_path, &stem, 5, tap);
    }
}

fn run_positive_test(
    tenor_path: &Path,
    expected_path: &Path,
    name: &str,
    category: &str,
    tap: &mut Tap,
) {
    let test_name = format!("{}/{}", category, name);

    let expected_json = match read_json(expected_path) {
        Ok(v) => v,
        Err(e) => {
            tap.not_ok(&test_name, format!("failed to read expected file: {}", e));
            return;
        }
    };

    match elaborate::elaborate(tenor_path) {
        Ok(got) => {
            if json_equal(&got, &expected_json) {
                tap.ok(&test_name);
            } else {
                let diff = json_diff(&expected_json, &got);
                tap.not_ok(&test_name, format!("output mismatch:\n{}", diff));
            }
        }
        Err(e) => {
            tap.not_ok(&test_name, format!(
                "unexpected elaboration error (pass {}): {}",
                e.pass, e.message
            ));
        }
    }
}

fn run_negative_test(
    tenor_path: &Path,
    expected_error_path: &Path,
    name: &str,
    pass: u8,
    tap: &mut Tap,
) {
    let test_name = format!("negative/pass{}/{}", pass, name);

    let expected_error = match read_json(expected_error_path) {
        Ok(v) => v,
        Err(e) => {
            tap.not_ok(&test_name, format!("failed to read expected-error file: {}", e));
            return;
        }
    };

    match elaborate::elaborate(tenor_path) {
        Err(got_error) => {
            let got_json = got_error.to_json_value();
            if json_equal(&got_json, &expected_error) {
                tap.ok(&test_name);
            } else {
                let diff = json_diff(&expected_error, &got_json);
                tap.not_ok(&test_name, format!("error mismatch:\n{}", diff));
            }
        }
        Ok(_) => {
            tap.not_ok(&test_name, format!(
                "expected pass {} elaboration error but elaboration succeeded",
                pass
            ));
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn glob_tenor_files(dir: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("tenor") {
                results.push(path);
            }
        }
    }
    results
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

fn read_json(path: &Path) -> Result<Value, String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    serde_json::from_str(&src)
        .map_err(|e| format!("invalid JSON in {}: {}", path.display(), e))
}

/// Deep equality of two JSON values, normalizing number types.
fn json_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Object(am), Value::Object(bm)) => {
            if am.len() != bm.len() { return false; }
            am.iter().all(|(k, v)| bm.get(k).map_or(false, |bv| json_equal(v, bv)))
        }
        (Value::Array(av), Value::Array(bv)) => {
            av.len() == bv.len() && av.iter().zip(bv).all(|(a, b)| json_equal(a, b))
        }
        (Value::Number(an), Value::Number(bn)) => {
            // Compare as f64 to handle i64 vs u64 representations
            an.as_f64() == bn.as_f64()
        }
        (Value::Null, Value::Null) => true,
        _ => a == b,
    }
}

/// Produce a human-readable diff between expected and got JSON values.
fn json_diff(expected: &Value, got: &Value) -> String {
    let exp_str = serde_json::to_string_pretty(expected).unwrap_or_default();
    let got_str = serde_json::to_string_pretty(got).unwrap_or_default();
    format!("--- expected\n{}\n+++ got\n{}", exp_str, got_str)
}
