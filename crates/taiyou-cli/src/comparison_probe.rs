//! Bounded parser safety probe for external crash inputs, not a render pass.
use std::{io::Read, path::Path};

/// Safe parse rejection is a successful safety result; I/O failures are not.
pub fn parse_file(path: &Path) -> Result<(), std::io::Error> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(std::io::Error::other("crash input exceeds probe budget"));
    }
    let options = taiyou_render::renderer::ParseOptions::default();
    let mut ingestion = taiyou_render::renderer::TrackIngestor::new();
    match taiyou_ass::parse_ass(&bytes, &options, &mut ingestion) {
        Ok(_) => println!("SAFETY_ONLY: parser returned an owned track"),
        Err(error) => println!("SAFETY_ONLY: parser rejected input: {error}"),
    }
    Ok(())
}
