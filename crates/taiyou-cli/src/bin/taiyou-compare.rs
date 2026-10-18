//! T015 subprocess adapter: typed C7 errors have fixed exit codes. Pixel results
//! are still read and computed by the harness, never returned as claimed JSON.
#[cfg(feature = "native")]
fn main() -> std::process::ExitCode {
    use std::{ffi::OsStr, process::ExitCode};
    use taiyou_render::renderer::ErrorCode;
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(OsStr::new("render")) {
        eprintln!("comparison adapter requires render");
        return ExitCode::from(80);
    }
    match taiyou_cli::render::run(args.collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}; event={:?} span={:?}", error.event(), error.span());
            ExitCode::from(match error.code() {
                ErrorCode::InvalidArgument => 10,
                ErrorCode::InvalidEncoding => 11,
                ErrorCode::InvalidScript => 12,
                ErrorCode::UnsupportedFeature => 20,
                ErrorCode::FontUnavailable => 30,
                ErrorCode::NativeFailure => 31,
                ErrorCode::LimitExceeded => 32,
                ErrorCode::AllocationFailed => 33,
                ErrorCode::InternalError => 34,
            })
        }
    }
}

#[cfg(not(feature = "native"))]
fn main() -> std::process::ExitCode {
    eprintln!("INFRA_ERROR: build comparison adapter with taiyou-cli/native");
    std::process::ExitCode::from(80)
}
