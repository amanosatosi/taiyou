//! Development command launcher. Execute only on CI runners under D0.

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, ExitCode};

mod inventory;
mod policy;

fn main() -> ExitCode {
    let Some(root) = Path::new(env!("CARGO_MANIFEST_DIR")).parent() else {
        eprintln!("xtask: workspace root is unavailable");
        return ExitCode::FAILURE;
    };
    let mut args = std::env::args_os().skip(1);
    let action = args.next();
    let rest: Vec<_> = args.collect();
    match action.as_deref() {
        Some(value) if value == OsStr::new("inventory") => return inventory::run(root, &rest),
        Some(value) if value == OsStr::new("policy") => return policy::run(root, &rest),
        Some(value) if value == OsStr::new("check-workspace") && rest.is_empty() => {}
        _ => {
            eprintln!("usage: cargo run -p xtask -- check-workspace | inventory ... | policy ... (CI runners only)");
            return ExitCode::from(2);
        }
    }
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
