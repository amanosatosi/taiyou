//! Development command launcher. Execute only on CI runners under D0.

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(OsStr::new("check-workspace")) || args.next().is_some() {
        eprintln!("usage: cargo run -p xtask -- check-workspace (CI runners only)");
        return ExitCode::from(2);
    }

    let Some(root) = Path::new(env!("CARGO_MANIFEST_DIR")).parent() else {
        eprintln!("xtask: workspace root is unavailable");
        return ExitCode::FAILURE;
    };
    match Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(root.join("scripts/check-workspace.ps1"))
        .current_dir(root)
        .status()
    {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("xtask: could not launch workspace validation: {error}");
            ExitCode::FAILURE
        }
    }
}
