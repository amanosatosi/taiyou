//! Unverified host font descriptors and session publication ownership.
//!
//! These shared containers do not authenticate bytes or native family metadata.
//! Production font resolution requires taiyou-font's sealed validated catalog.
use crate::*;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct FontBlob {
    id: FontId,
    bytes: Arc<[u8]>,
}
impl FontBlob {
    /// Creates an explicitly unverified descriptor, not a validated font identity.
    pub fn from_unverified(id: FontId, bytes: Arc<[u8]>) -> Self {
        Self { id, bytes }
    }
    pub const fn id(&self) -> FontId {
        self.id
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn shared_bytes(&self) -> Arc<[u8]> {
        self.bytes.clone()
    }
}
#[derive(Clone, Debug)]
pub struct FontRecord {
    blob: FontBlob,
    family: Arc<str>,
    folded_family: Arc<str>,
    weight: u16,
    italic: bool,
}
impl FontRecord {
    /// Nonempty names are structural checks only. Native intake must verify all
    /// metadata against the actual bytes before any production font resolution.
    pub fn from_unverified(
        blob: FontBlob,
        family: Arc<str>,
        folded_family: Arc<str>,
        weight: u16,
        italic: bool,
    ) -> Result<Self, FontError> {
        if family.is_empty() || folded_family.is_empty() {
            return Err(FontError::new(
                ErrorCode::FontUnavailable,
                "font family is empty",
            ));
        }
        Ok(Self {
            blob,
            family,
            folded_family,
            weight,
            italic,
        })
    }
    pub fn blob(&self) -> &FontBlob {
        &self.blob
    }
    pub fn family(&self) -> &str {
        &self.family
    }
    pub fn folded_family(&self) -> &str {
        &self.folded_family
    }
    pub const fn weight(&self) -> u16 {
        self.weight
    }
    pub const fn italic(&self) -> bool {
        self.italic
    }
}
#[derive(Clone, Debug)]
/// Immutable unverified descriptors; not a native-validated runtime catalog.
pub struct FontCatalog {
    revision: CatalogRevision,
    faces: Arc<[FontRecord]>,
}
impl FontCatalog {
    pub const fn revision(&self) -> CatalogRevision {
        self.revision
    }
    pub fn faces(&self) -> &[FontRecord] {
        &self.faces
    }
}
/// One non-cloneable publisher per catalog session; no global/pointer identity.
#[derive(Debug, Default)]
pub struct CatalogIngestor {
    next: Option<u64>,
    initialized: bool,
}
impl CatalogIngestor {
    pub fn new() -> Self {
        Self::default()
    }
    /// Publishes descriptors without authenticating their claimed identities or
    /// metadata. Production code must pass these through native validation.
    pub fn publish_unverified(
        &mut self,
        faces: Arc<[FontRecord]>,
    ) -> Result<Arc<FontCatalog>, FontError> {
        let value = if self.initialized {
            self.next.ok_or_else(|| {
                FontError::new(ErrorCode::LimitExceeded, "catalog revision exhausted")
            })?
        } else {
            0
        };
        self.initialized = true;
        self.next = value.checked_add(1);
        Ok(Arc::new(FontCatalog {
            revision: CatalogRevision(value),
            faces,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_revision_exhaustion_does_not_recycle() {
        let mut owner = CatalogIngestor {
            next: Some(u64::MAX),
            initialized: true,
        };
        assert_eq!(
            owner
                .publish_unverified(Arc::from([]))
                .unwrap()
                .revision()
                .get(),
            u64::MAX
        );
        for _ in 0..2 {
            assert_eq!(
                owner.publish_unverified(Arc::from([])).unwrap_err().code(),
                ErrorCode::LimitExceeded
            );
        }
    }
}
