//! Explicit-font CoreV1 CLI and C6 straight-alpha PNG export.
use std::{
    ffi::OsString,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use taiyou_render::renderer::*;

mod output;

fn invalid(detail: &str) -> RenderError {
    RenderError::new(ErrorCode::InvalidArgument, detail)
}
fn io_error(e: std::io::Error) -> RenderError {
    invalid(&format!("file I/O: {e}"))
}
fn parse_error(e: ParseError) -> RenderError {
    RenderError::new(e.code(), e.detail()).with_context(e.event(), e.span())
}
fn font_error(e: FontError) -> RenderError {
    let mut result = RenderError::new(e.code(), e.detail()).with_context(e.event(), e.span());
    if let Some(code) = e.native_code() {
        result = result.with_native_code(code);
    }
    result
}
fn number<T: std::str::FromStr>(value: &std::ffi::OsStr) -> Result<T, RenderError> {
    value
        .to_str()
        .ok_or_else(|| invalid("numeric value is not UTF-8"))?
        .parse()
        .map_err(|_| invalid("invalid or out-of-range integer"))
}
#[derive(Debug)]
struct Arguments {
    input: PathBuf,
    fonts: Vec<PathBuf>,
    time: i64,
    width: u32,
    height: u32,
    output: PathBuf,
    raw: Option<PathBuf>,
    policy: UnsupportedPolicy,
}
impl Arguments {
    fn parse(args: Vec<OsString>) -> Result<Self, RenderError> {
        let mut values = std::collections::BTreeMap::new();
        let mut fonts = Vec::new();
        let mut args = args.into_iter();
        while let Some(key) = args.next() {
            let key = key.to_str().ok_or_else(|| invalid("flag is not UTF-8"))?;
            if ![
                "--input",
                "--font",
                "--time-us",
                "--width",
                "--height",
                "--profile",
                "--output",
                "--raw",
                "--unsupported",
            ]
            .contains(&key)
            {
                return Err(invalid("unknown render flag"));
            }
            let value = args
                .next()
                .ok_or_else(|| invalid("flag requires a value"))?;
            if key == "--font" {
                fonts.try_reserve(1).map_err(|_| {
                    RenderError::new(
                        ErrorCode::AllocationFailed,
                        "font argument allocation failed",
                    )
                })?;
                fonts.push(PathBuf::from(value));
            } else if values.insert(key.to_owned(), value).is_some() {
                return Err(invalid("duplicate render flag"));
            }
        }
        let get = |key| {
            values
                .get(key)
                .ok_or_else(|| invalid("missing required render flag"))
        };
        if get("--profile")? != "core-v1" {
            return Err(RenderError::new(
                ErrorCode::UnsupportedFeature,
                "only explicit core-v1 is available",
            ));
        }
        if fonts.is_empty() {
            return Err(RenderError::new(
                ErrorCode::FontUnavailable,
                "at least one explicit --font is required",
            ));
        }
        let policy_value = match values.get("--unsupported") {
            Some(s) => s
                .to_str()
                .ok_or_else(|| invalid("unsupported policy is not UTF-8"))?,
            None => "strict",
        };
        let policy = match policy_value {
            "strict" => UnsupportedPolicy::Strict,
            "tolerant" => UnsupportedPolicy::Tolerant,
            _ => return Err(invalid("unsupported policy must be strict or tolerant")),
        };
        let input = PathBuf::from(get("--input")?);
        let output = PathBuf::from(get("--output")?);
        let raw = values.get("--raw").map(PathBuf::from);
        if input == output
            || raw.as_ref() == Some(&input)
            || raw.as_ref() == Some(&output)
            || fonts.contains(&output)
            || raw.as_ref().is_some_and(|p| fonts.contains(p))
        {
            return Err(invalid(
                "output paths must differ from inputs and each other",
            ));
        }
        Ok(Self {
            input,
            fonts,
            time: number(get("--time-us")?)?,
            width: number(get("--width")?)?,
            height: number(get("--height")?)?,
            output,
            raw,
            policy,
        })
    }
}
fn read_limited(path: &Path, cap: u64, inputs: &mut Vec<File>) -> Result<Vec<u8>, RenderError> {
    let mut file = output::open_input(path).map_err(io_error)?;
    if file.metadata().map_err(io_error)?.len() > cap {
        return Err(RenderError::new(
            ErrorCode::LimitExceeded,
            "input file budget exceeded",
        ));
    }
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        let n = file.read(&mut buffer).map_err(io_error)?;
        if n == 0 {
            break;
        }
        if (bytes.len() as u64)
            .checked_add(n as u64)
            .is_none_or(|n| n > cap)
        {
            return Err(RenderError::new(
                ErrorCode::LimitExceeded,
                "input file grew beyond budget",
            ));
        }
        bytes.try_reserve_exact(n).map_err(|_| {
            RenderError::new(ErrorCode::AllocationFailed, "file buffer allocation failed")
        })?;
        bytes.extend_from_slice(&buffer[..n]);
    }
    inputs.try_reserve(1).map_err(|_| {
        RenderError::new(
            ErrorCode::AllocationFailed,
            "input handle allocation failed",
        )
    })?;
    inputs.push(file);
    Ok(bytes)
}
/// Does not mutate or borrow pixels beyond this call. No color-space transform.
pub fn straight_rgba(frame: &Frame, limits: &Limits) -> Result<Vec<u8>, RenderError> {
    let n = frame.pixels().len();
    if n as u64 > limits.transient_bytes() {
        return Err(RenderError::new(
            ErrorCode::LimitExceeded,
            "PNG conversion budget exceeded",
        ));
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(n).map_err(|_| {
        RenderError::new(
            ErrorCode::AllocationFailed,
            "PNG conversion allocation failed",
        )
    })?;
    for p in frame.pixels().chunks_exact(4) {
        let a = u32::from(p[3]);
        for c in &p[..3] {
            bytes.push(
                (u32::from(*c) * 255 + a / 2)
                    .checked_div(a)
                    .unwrap_or(0)
                    .min(255) as u8,
            );
        }
        bytes.push(p[3]);
    }
    Ok(bytes)
}
pub fn write_png<W: Write>(frame: &Frame, writer: W, limits: &Limits) -> Result<(), RenderError> {
    // Bounded conversion and conservative pure-Rust codec scratch reservation.
    let peak = (frame.pixels().len() as u64)
        .checked_mul(8)
        .and_then(|n| n.checked_add(4 * 1024 * 1024));
    if peak.is_none_or(|n| n > limits.transient_bytes()) {
        return Err(RenderError::new(
            ErrorCode::LimitExceeded,
            "PNG encoder reservation exceeded",
        ));
    }
    let bytes = straight_rgba(frame, limits)?;
    let mut encoder = png::Encoder::new(writer, frame.size().width(), frame.size().height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let fail = |e: png::EncodingError| invalid(&format!("PNG encoding: {e}"));
    let mut png = encoder.write_header().map_err(fail)?;
    png.write_image_data(&bytes).map_err(fail)?;
    png.finish().map_err(fail)
}
pub fn run(args: Vec<OsString>) -> Result<(), RenderError> {
    let args = Arguments::parse(args)?;
    let config = RendererConfig::default();
    let limits = &config.limits;
    let request = RenderRequest {
        now: TimeUs::from_micros(args.time),
        viewport: Viewport::full(Size::new(args.width, args.height))?,
        unsupported: args.policy,
    };
    limits
        .check_output_size(request.viewport.size())
        .map_err(parse_error)?;
    let options = ParseOptions::new(
        InputEncoding::Utf8,
        Profile::CoreV1,
        limits.clone(),
        SyntaxPolicy::Strict,
    );
    // Retain the actual input handles until publication, including Windows
    // sharing protection against hard links, short names and other aliases.
    let mut inputs = Vec::new();
    let parsed = taiyou_ass::parse_ass(
        &read_limited(&args.input, limits.input_bytes(), &mut inputs)?,
        &options,
        &mut TrackIngestor::new(),
    )
    .map_err(parse_error)?;
    let mut context = FontContext::with_limits(limits.clone()).map_err(font_error)?;
    let mut builder = FontCatalogBuilder::new(limits.clone());
    for path in &args.fonts {
        builder
            .register(
                &mut context,
                &read_limited(path, limits.font_bytes(), &mut inputs)?,
                0,
            )
            .map_err(font_error)?;
    }
    let catalog = builder
        .snapshot(&mut CatalogIngestor::new())
        .map_err(font_error)?;
    drop(builder);
    drop(context);
    let frame = Renderer::new(config.clone())?.render_parsed(&parsed, catalog, &request)?;
    for d in frame.diagnostics() {
        eprintln!(
            "{} event={:?} span={:?}: {}",
            d.code().as_str(),
            d.event(),
            d.span(),
            d.detail()
        );
    }
    output::write(&args.output, args.raw.as_deref(), &inputs, |png, raw| {
        write_png(&frame, png, limits)?;
        if let Some(raw) = raw {
            raw.write_all(frame.pixels()).map_err(io_error)?;
        }
        Ok(())
    })?;
    eprintln!(
        "taiyou {} profile=core-v1 workers=1 RGBA8 premultiplied {}x{} time-us={} diagnostics={}",
        crate::VERSION,
        args.width,
        args.height,
        args.time,
        frame.diagnostics().len()
    );
    Ok(())
}
