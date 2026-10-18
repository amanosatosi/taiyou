//! No authenticated xy captures exist yet. Never fabricate capture evidence.
use std::{ffi::OsString, path::Path, process::ExitCode};

pub fn run(_root: &Path, _args: &[OsString]) -> ExitCode {
    eprintln!(
        "INFRA_ERROR: authenticated xy capture and environment metadata unavailable; import not certified"
    );
    ExitCode::FAILURE
}
