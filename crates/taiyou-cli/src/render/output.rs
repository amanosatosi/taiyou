//! Same-directory staging; no destination is ever opened with truncation.
//!
//! All generation, flushes and syncs finish before publication starts. Each
//! rename publishes a complete file; two separate destinations are not a single
//! filesystem transaction. An error during the second rename can therefore
//! leave the first existing destination replaced with its complete new file.
use super::{RenderError, invalid, io_error};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

pub(super) fn open_input(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    // Stable since Rust 1.10. The kernel applies sharing to the actual file,
    // not its spelling, including hard links and DOS short-name aliases.
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1); // FILE_SHARE_READ; deny writes and deletion.
    }
    options.open(path)
}

fn destination(path: &Path) -> Result<PathBuf, RenderError> {
    let name = path.file_name().ok_or_else(|| invalid("output needs a file name"))?;
    #[cfg(windows)]
    {
        // Do not allow alternate streams, device names or Win32 trimming
        // aliases, even when a caller supplies an extended-length path.
        let name = name.to_str().ok_or_else(|| invalid("invalid Windows output name"))?;
        let stem = name.split('.').next().unwrap_or_default().to_uppercase();
        let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$")
            || ["COM", "LPT"].iter().any(|prefix| {
                stem.strip_prefix(prefix).is_some_and(|n| {
                    matches!(n, "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³")
                })
            });
        if device || name.ends_with(['.', ' '])
            || name.chars().any(|c| c.is_control() || ":<>\"|?*".contains(c)) {
            return Err(invalid("unsafe Windows output name"));
        }
    }
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    Ok(parent.canonicalize().map_err(io_error)?.join(name))
}

fn probe(path: &Path, protected: &[File]) -> Result<Option<File>, RenderError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(invalid("output must be a regular file, not a symlink or directory"));
        }
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io_error(e)),
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true); // Never create, truncate or write through it.
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // A probe cannot coexist with an input guard or another output probe
        // for the same file. This avoids unstable Windows file-ID APIs/FFI.
        options.share_mode(1); // FILE_SHARE_READ
        let _ = protected;
    }
    let file = options.open(path).map_err(io_error)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let a = file.metadata().map_err(io_error)?;
        for other in protected {
            let b = other.metadata().map_err(io_error)?;
            if a.dev() == b.dev() && a.ino() == b.ino() {
                return Err(invalid("output aliases an input or another output"));
            }
        }
    }
    Ok(Some(file))
}

struct Staged {
    destination: PathBuf,
    temporary: Option<PathBuf>,
    file: Option<File>,
    remove_published: bool,
}
impl Staged {
    fn new(destination: PathBuf, destinations: &[PathBuf]) -> Result<Self, RenderError> {
        let parent = destination.parent().ok_or_else(|| invalid("missing output parent"))?;
        for _ in 0..128 {
            let n = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let temporary = parent.join(format!(".taiyou-{}-{n}.tmp", std::process::id()));
            // The generated temporary name must not itself be a destination.
            if destinations.contains(&temporary) {
                continue;
            }
            match OpenOptions::new().write(true).create_new(true).open(&temporary) {
                Ok(file) => return Ok(Self {
                    destination, temporary: Some(temporary), file: Some(file), remove_published: false,
                }),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(io_error(e)),
            }
        }
        Err(invalid("cannot reserve an unused temporary output"))
    }
    fn finish(&mut self) -> Result<(), RenderError> {
        if let Some(mut file) = self.file.take() {
            file.flush().map_err(io_error)?;
            file.sync_all().map_err(io_error)?;
        }
        Ok(())
    }
}
impl Drop for Staged {
    fn drop(&mut self) {
        drop(self.file.take()); // Close before unlinking on Windows.
        if let Some(path) = &self.temporary {
            let _ = fs::remove_file(path);
        }
        if self.remove_published {
            let _ = fs::remove_file(&self.destination);
        }
    }
}

fn publish(outputs: &mut [Staged], inputs: &[File]) -> Result<(), RenderError> {
    // Keep all probes alive together: Windows detects aliases by sharing;
    // Unix compares dev/inode, including hard-linked output destinations.
    let mut guards: Vec<File> = inputs.iter().map(File::try_clone).collect::<io::Result<_>>().map_err(io_error)?;
    for output in outputs.iter() {
        if let Some(file) = probe(&output.destination, &guards)? {
            guards.push(file);
        }
    }
    drop(guards);
    let mut published: Vec<File> = inputs.iter().map(File::try_clone).collect::<io::Result<_>>().map_err(io_error)?;
    for output in outputs.iter_mut() {
        // Recheck at publication, also against already published output guards.
        // This catches platform aliases of previously nonexistent names (case
        // folding/short names) without guessing the filesystem's case table.
        let previous = probe(&output.destination, &published)?;
        let new = previous.is_none();
        drop(previous);
        let temporary = output.temporary.as_ref().ok_or_else(|| invalid("missing staged output"))?;
        fs::rename(temporary, &output.destination).map_err(io_error)?;
        output.temporary = None;
        output.remove_published = new;
        published.push(open_input(&output.destination).map_err(io_error)?);
    }
    for output in outputs {
        output.remove_published = false;
    }
    Ok(())
}

pub(super) fn write(
    png: &Path,
    raw: Option<&Path>,
    inputs: &[File],
    generate: impl FnOnce(&mut File, Option<&mut File>) -> Result<(), RenderError>,
) -> Result<(), RenderError> {
    let mut destinations = vec![destination(png)?];
    if let Some(raw) = raw {
        let raw = destination(raw)?;
        if raw == destinations[0] {
            return Err(invalid("PNG and raw destinations alias"));
        }
        destinations.push(raw);
    }
    let mut outputs = Vec::new();
    for path in &destinations {
        outputs.push(Staged::new(path.clone(), &destinations)?);
    }
    let (png, raw) = outputs.split_at_mut(1);
    let png = png[0].file.as_mut().ok_or_else(|| invalid("missing PNG staging file"))?;
    let raw = raw.first_mut().and_then(|r| r.file.as_mut());
    generate(png, raw)?;
    for output in &mut outputs {
        output.finish()?;
    }
    publish(&mut outputs, inputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Directory(PathBuf);
    impl Drop for Directory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn partial_encoder_and_raw_writer_failures_preserve_old_files() {
        let directory = Directory(std::env::temp_dir().join(format!(
            "taiyou-output-unit-{}-{}", std::process::id(), NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        )));
        fs::create_dir(&directory.0).unwrap();
        let png = directory.0.join("frame.png");
        let raw = directory.0.join("frame.rgba");
        for fail_raw in [false, true] {
            fs::write(&png, b"old PNG").unwrap();
            fs::write(&raw, b"old raw").unwrap();
            let result = write(&png, Some(&raw), &[], |png, raw| {
                png.write_all(b"partial PNG").map_err(io_error)?;
                if fail_raw {
                    raw.unwrap().write_all(b"partial raw").map_err(io_error)?;
                }
                Err(io_error(io::Error::new(io::ErrorKind::WriteZero, "injected writer failure")))
            });
            assert!(result.is_err());
            assert_eq!(fs::read(&png).unwrap(), b"old PNG");
            assert_eq!(fs::read(&raw).unwrap(), b"old raw");
            assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 2);
        }
    }

    #[test]
    fn failed_generation_does_not_publish_new_destinations() {
        let directory = Directory(std::env::temp_dir().join(format!(
            "taiyou-output-new-unit-{}-{}", std::process::id(), NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        )));
        fs::create_dir(&directory.0).unwrap();
        let png = directory.0.join("frame.png");
        let raw = directory.0.join("frame.rgba");
        assert!(write(&png, Some(&raw), &[], |png, _| {
            png.write_all(b"partial PNG").map_err(io_error)?;
            Err(invalid("injected encoding failure"))
        }).is_err());
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 0);
    }
}
