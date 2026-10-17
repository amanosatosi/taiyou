use crate::native::FontContext;
use icu_casemap::CaseMapper;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use taiyou_types::*;
fn error(code: ErrorCode, detail: &str) -> FontError {
    FontError::new(code, detail)
}
pub fn fold_family(name: &str, limits: &Limits) -> Result<String, FontError> {
    let cost = u64::try_from(name.len())
        .ok()
        .and_then(|n| n.checked_mul(3))
        .ok_or_else(|| error(ErrorCode::LimitExceeded, "family fold length overflow"))?;
    if cost > limits.transient_bytes() {
        return Err(error(
            ErrorCode::LimitExceeded,
            "family fold transient budget exceeded",
        ));
    }
    Ok(CaseMapper::new().fold_string(name).into_owned())
}
pub struct FontCatalogBuilder {
    limits: Limits,
    records: Vec<FontRecord>,
    blobs: Vec<([u8; 32], Arc<[u8]>)>,
    bytes: u64,
    names: u64,
    publisher: CatalogIngestor,
}
impl FontCatalogBuilder {
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            records: Vec::new(),
            blobs: Vec::new(),
            bytes: 0,
            names: 0,
            publisher: CatalogIngestor::new(),
        }
    }
    pub const fn registered_bytes(&self) -> u64 {
        self.bytes
    }
    pub fn register(
        &mut self,
        context: &mut FontContext,
        bytes: &[u8],
        face_index: u32,
    ) -> Result<FontId, FontError> {
        let length = u64::try_from(bytes.len())
            .map_err(|_| error(ErrorCode::LimitExceeded, "font length overflow"))?;
        if length == 0 || length > self.limits.font_bytes() {
            return Err(error(ErrorCode::LimitExceeded, "font byte budget exceeded"));
        }
        let hash: [u8; 32] = Sha256::digest(bytes).into();
        let id = FontId::new(hash, face_index);
        let existing = self.blobs.iter().position(|(h, _)| *h == hash);
        if let Some(index) = existing {
            if self.blobs[index].1.as_ref() != bytes {
                return Err(error(ErrorCode::InternalError, "font digest collision"));
            }
        }
        if self.records.iter().any(|record| record.blob().id() == id) {
            return Ok(id);
        }
        let new_total = self
            .bytes
            .checked_add(if existing.is_some() { 0 } else { length })
            .ok_or_else(|| error(ErrorCode::LimitExceeded, "font budget overflow"))?;
        if new_total > self.limits.font_bytes() {
            return Err(error(
                ErrorCode::LimitExceeded,
                "registered font byte budget exceeded",
            ));
        }
        let metadata_cost = u64::try_from(self.records.len())
            .ok()
            .and_then(|n| n.checked_add(1))
            .and_then(|n| n.checked_mul(3 * std::mem::size_of::<FontRecord>() as u64))
            .ok_or_else(|| error(ErrorCode::LimitExceeded, "catalog metadata overflow"))?;
        // Includes native scratch estimate, name decoding/folding, copy-to-Arc
        // overlap, retained names and catalog publication overlap. Native RSS is
        // measured separately in R003; this is the explicit C7 reservation.
        let intake_cost = length
            .checked_mul(12)
            .and_then(|n| n.checked_add(64 * 1024))
            .and_then(|n| n.checked_add(metadata_cost))
            .and_then(|n| n.checked_add(self.names))
            .ok_or_else(|| error(ErrorCode::LimitExceeded, "font intake size overflow"))?;
        if intake_cost > self.limits.transient_bytes() {
            return Err(error(
                ErrorCode::LimitExceeded,
                "font intake transient budget exceeded",
            ));
        }
        let data = if let Some(index) = existing {
            self.blobs[index].1.clone()
        } else {
            let mut owned = Vec::new();
            owned
                .try_reserve_exact(bytes.len())
                .map_err(|_| error(ErrorCode::AllocationFailed, "font copy allocation failed"))?;
            owned.extend_from_slice(bytes);
            Arc::from(owned)
        };
        let metadata = context.inspect(data.clone(), face_index)?;
        let folded = fold_family(&metadata.family, &self.limits)?;
        let names = self.names
            .checked_add(metadata.family.len() as u64)
            .and_then(|n| n.checked_add(folded.len() as u64))
            .ok_or_else(|| error(ErrorCode::LimitExceeded, "catalog name size overflow"))?;
        let record = FontRecord::new(
            FontBlob::new(id, data.clone()),
            Arc::from(metadata.family),
            Arc::from(folded),
            metadata.weight,
            metadata.italic,
        )?;
        self.records
            .try_reserve(1)
            .map_err(|_| error(ErrorCode::AllocationFailed, "catalog allocation failed"))?;
        if existing.is_none() {
            self.blobs.try_reserve(1).map_err(|_| {
                error(
                    ErrorCode::AllocationFailed,
                    "catalog blob allocation failed",
                )
            })?;
            self.blobs.push((hash, data));
        }
        self.records.push(record);
        self.bytes = new_total;
        self.names = names;
        Ok(id)
    }
    pub fn snapshot(&mut self) -> Result<Arc<FontCatalog>, FontError> {
        let mut records = Vec::new();
        records.try_reserve_exact(self.records.len()).map_err(|_| {
            error(
                ErrorCode::AllocationFailed,
                "catalog snapshot allocation failed",
            )
        })?;
        records.extend(self.records.iter().cloned());
        self.publisher.publish(records.into())
    }
}
pub fn resolve_family(
    catalog: &FontCatalog,
    request: &FontRequest,
    limits: &Limits,
) -> Result<FontId, FontError> {
    if request.vertical() || request.family().starts_with('@') {
        return Err(error(
            ErrorCode::UnsupportedFeature,
            "vertical font requests remain gated",
        ));
    }
    let folded = fold_family(request.family(), limits)?;
    catalog
        .faces()
        .iter()
        .enumerate()
        .filter(|(_, record)| record.folded_family() == folded)
        .min_by_key(|(index, record)| {
            (
                record.weight().abs_diff(request.weight()),
                record.italic() != request.italic(),
                *index,
            )
        })
        .map(|(_, record)| record.blob().id())
        .ok_or_else(|| {
            error(
                ErrorCode::FontUnavailable,
                "requested family is absent from host catalog",
            )
        })
}
