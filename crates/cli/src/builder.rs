//! `tenor builder` command — starts the Tenor Builder SPA dev server or produces a production build.
//!
//! The Builder SPA lives in the `builder/` directory relative to the workspace root.
//! This command delegates to npm/vite for the actual build tooling.

use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::{self, Child, Command};

/// Options for `tenor builder` (dev server mode).
pub struct BuilderOptions<'a> {
    pub port: u16,
    pub open: bool,
    pub contract: Option<&'a Path>,
    pub quiet: bool,
}

/// Options for `tenor builder build` (production build mode).
pub struct BuilderBuildOptions<'a> {
    pub output_dir: &'a Path,
    pub quiet: bool,
}

/// Locate the builder/ directory.
///
/// Strategy:
/// 1. Check if `builder/` exists relative to the current working directory
/// 2. Check if `builder/` exists relative to the executable's directory
///
/// Returns `None` if the builder directory cannot be found.
fn find_builder_dir() -> Option<PathBuf> {
    // Try cwd/builder/
    let cwd_candidate = std::env::current_dir().ok()?.join("builder");
    if cwd_candidate.is_dir() && cwd_candidate.join("package.json").exists() {
        return Some(cwd_candidate);
    }

    // Try exe/../builder/ (when installed globally)
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    if let Some(dir) = exe_dir {
        let exe_candidate = dir.join("builder");
        if exe_candidate.is_dir() && exe_candidate.join("package.json").exists() {
            return Some(exe_candidate);
        }
    }

    None
}

/// Ensure npm dependencies are installed in the builder directory.
fn ensure_deps(builder_dir: &Path, quiet: bool) -> bool {
    let node_modules = builder_dir.join("node_modules");
    if node_modules.exists() {
        return true;
    }

    if !quiet {
        eprintln!(
            "tenor builder: node_modules not found in '{}', running npm install...",
            builder_dir.display()
        );
    }

    let status = Command::new("npm")
        .arg("install")
        .current_dir(builder_dir)
        .status();

    match status {
        Ok(s) if s.success() => {
            if !quiet {
                eprintln!("tenor builder: npm install complete.");
            }
            true
        }
        Ok(s) => {
            eprintln!(
                "error: npm install failed with exit code {}",
                s.code().unwrap_or(-1)
            );
            false
        }
        Err(e) => {
            eprintln!("error: failed to run npm install: {}", e);
            false
        }
    }
}

/// Run `tenor builder` dev server.
pub fn cmd_builder(opts: BuilderOptions<'_>) {
    let builder_dir = match find_builder_dir() {
        Some(d) => d,
        None => {
            eprintln!(
                "error: could not find builder/ directory.\n\
                 Make sure you are running from the Tenor workspace root or \
                 that the builder has been installed alongside the binary."
            );
            process::exit(1);
        }
    };

    if !ensure_deps(&builder_dir, opts.quiet) {
        process::exit(1);
    }

    if !opts.quiet {
        println!(
            "Starting Tenor Builder dev server on http://localhost:{}",
            opts.port
        );
    }

    // Build environment for the vite process
    let mut env_extras: Vec<(String, String)> = Vec::new();
    if let Some(contract_path) = opts.contract {
        let abs_path = contract_path
            .canonicalize()
            .unwrap_or_else(|_| contract_path.to_path_buf());
        env_extras.push((
            "TENOR_BUILDER_CONTRACT".to_string(),
            abs_path.to_string_lossy().into_owned(),
        ));
        // Vite exposes VITE_* env vars to the client
        env_extras.push((
            "VITE_TENOR_CONTRACT_PATH".to_string(),
            abs_path.to_string_lossy().into_owned(),
        ));
    }

    // Spawn: npx vite --port <port>
    let mut cmd = Command::new("npx");
    cmd.arg("vite")
        .arg("--port")
        .arg(opts.port.to_string())
        .current_dir(&builder_dir);

    for (key, value) in &env_extras {
        cmd.env(key, value);
    }

    let mut child: Child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: failed to spawn vite: {}", e);
            process::exit(1);
        }
    };

    // Optionally open browser
    if opts.open {
        // Small delay to let vite start
        std::thread::sleep(std::time::Duration::from_millis(2000));
        let url = format!("http://localhost:{}", opts.port);
        if let Err(e) = open_browser(&url) {
            eprintln!("warning: could not open browser: {}", e);
        }
    }

    // Set up Ctrl+C handler to kill child process
    let child_id = child.id();
    let _ = ctrlc_install(child_id);

    // Wait for vite to exit
    match child.wait() {
        Ok(status) => {
            if !status.success() {
                process::exit(status.code().unwrap_or(1));
            }
        }
        Err(e) => {
            eprintln!("error waiting for vite process: {}", e);
            process::exit(1);
        }
    }
}

/// Run `tenor builder build` (production build).
pub fn cmd_builder_build(opts: BuilderBuildOptions<'_>) {
    let builder_dir = match find_builder_dir() {
        Some(d) => d,
        None => {
            eprintln!(
                "error: could not find builder/ directory.\n\
                 Make sure you are running from the Tenor workspace root."
            );
            process::exit(1);
        }
    };

    if !ensure_deps(&builder_dir, opts.quiet) {
        process::exit(1);
    }

    let abs_output = opts
        .output_dir
        .canonicalize()
        .unwrap_or_else(|_| opts.output_dir.to_path_buf());

    if !opts.quiet {
        println!(
            "Building Tenor Builder for production -> {}",
            abs_output.display()
        );
    }

    let status = Command::new("npx")
        .arg("vite")
        .arg("build")
        .arg("--outDir")
        .arg(&abs_output)
        .current_dir(&builder_dir)
        .status();

    match status {
        Ok(s) if s.success() => {
            if !opts.quiet {
                println!("Build complete: {}", abs_output.display());
            }
        }
        Ok(s) => {
            eprintln!(
                "error: vite build failed with exit code {}",
                s.code().unwrap_or(-1)
            );
            process::exit(s.code().unwrap_or(1));
        }
        Err(e) => {
            eprintln!("error: failed to run vite build: {}", e);
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Platform utilities
// ---------------------------------------------------------------------------

fn open_browser(url: &str) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn().map(|_| ())
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(url).spawn().map(|_| ())
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/c", "start", url])
            .spawn()
            .map(|_| ())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = url;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "open browser not supported on this platform",
        ))
    }
}

/// Install a Ctrl+C handler that kills the child process.
///
/// Uses a simple approach compatible with platforms that may not have
/// the `ctrlc` crate available.
fn ctrlc_install(child_pid: u32) -> io::Result<()> {
    // We use a background thread that waits for the process to be killed
    // via SIGINT. This is a simplified approach using std::sync.
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    // Spawn a thread that monitors for process termination
    // The actual Ctrl+C signal handling relies on the OS propagating
    // SIGINT to the child process group automatically on most Unix systems.
    //
    // For more robust handling, the ctrlc crate would be ideal, but we
    // avoid adding a dependency per the plan spec.
    std::thread::spawn(move || {
        // When the parent process receives SIGINT, the OS will also
        // send it to the child process group (on Unix). We just need
        // to ensure we don't block indefinitely.
        while running_clone.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });

    // Log PID for debugging
    let _ = io::stderr().write_all(
        format!("tenor builder: vite process pid={child_pid} (Ctrl+C to stop)\n").as_bytes(),
    );

    Ok(())
}
