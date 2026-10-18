//! Shared runner-only process launcher; reports are computed by the harness.
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, ExitCode};

pub fn launch(root: &Path, script: &str, args: &[OsString]) -> ExitCode {
    match Command::new("python")
        .arg(root.join("validation").join(script))
        .args(args)
        .current_dir(root)
        .status()
    {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("xtask {script}: {error}");
            ExitCode::FAILURE
        }
    }
}
