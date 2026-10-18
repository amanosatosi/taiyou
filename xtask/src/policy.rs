//! Candidate policy tests; this launcher cannot establish a trusted check issuer.
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, ExitCode};

pub fn run(root: &Path, args: &[OsString]) -> ExitCode {
    match Command::new("python")
        .arg(root.join("validation/policy.py"))
        .args(args)
        .current_dir(root)
        .status()
    {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("xtask policy: {error}");
            ExitCode::FAILURE
        }
    }
}
