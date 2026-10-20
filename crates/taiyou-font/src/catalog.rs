use crate::native::FontContext;
use icu_casemap::CaseMapper;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use taiyou_types::*;
fn error(code: ErrorCode, detail: &str) -> FontError {
    FontError::new(code, detail)
}
/// Immutable native-validated publication. Only this module can construct it;
/// shared unverified descriptors cannot be converted without validation.
/// ```compile_fail
/// use taiyou_types::{FontCatalog, FontRequest, Limits};
/// fn forged(draft: &FontCatalog, request: &FontRequest) {
///     taiyou_font::resolve_family(draft, request, &Limits::default());
/// }
/// ```
/// ```compile_fail
/// fn forged(raw: std::sync::Arc<taiyou_types::FontCatalog>) {
///     let catalog = taiyou_font::ValidatedFontCatalog { inner: raw };
/// }
/// ```
#[derive(Clone, Debug)]
pub struct ValidatedFontCatalog {
    inner: Arc<FontCatalog>,
    // Folded SFNT family/typographic-family names, in native record order.
    // One entry per validated face; aliases never mint another FontId.
    aliases: Arc<[Arc<[String]>]>,
}
impl ValidatedFontCatalog {
    pub fn revision(&self) -> CatalogRevision {
        self.inner.revision()
    }
    pub fn faces(&self) -> &[FontRecord] {
        self.inner.faces()
    }
    pub(crate) fn matches_family(&self, index: usize, folded: &str) -> bool {
        self.faces()[index].folded_family() == folded
            || self.aliases[index]
                .iter()
                .any(|alias| alias.as_str() == folded)
    }
    /// Extract through the immutable catalog's authentication proof. Arbitrary
    /// public FontBlob inputs still use FontContext::outline and reauthenticate.
    pub fn outline(
        &self,
        context: &mut FontContext,
        font: FontId,
        glyph: u32,
        limits: &Limits,
    ) -> Result<GlyphOutline, FontError> {
        let face = self
            .faces()
            .iter()
            .find(|face| face.blob().id() == font)
            .ok_or_else(|| {
                error(
                    ErrorCode::InvalidArgument,
                    "font absent from validated catalog",
                )
            })?;
        context.outline_authenticated(face.blob(), glyph, limits)
    }
    pub(crate) fn shape_raw(
        &self,
        context: &mut FontContext,
        font: FontId,
        request: &crate::shape::NativeRequest<'_>,
        limits: &Limits,
        collect_ink: bool,
    ) -> Result<crate::shape::RawRun, FontError> {
        let face = self
            .faces()
            .iter()
            .find(|face| face.blob().id() == font)
            .ok_or_else(|| {
                error(
                    ErrorCode::InvalidArgument,
                    "font absent from validated catalog",
                )
            })?;
        context.shape_authenticated_with_ink(face.blob(), request, limits, collect_ink)
    }
}
pub(crate) fn check_blob_identity(blob: &FontBlob) -> Result<(), FontError> {
    let actual: [u8; 32] = Sha256::digest(blob.bytes()).into();
    if actual != *blob.id().sha256() {
        return Err(error(
            ErrorCode::InvalidArgument,
            "font bytes do not match claimed SHA-256",
        ));
    }
    Ok(())
}
pub fn fold_family(name: &str, limits: &Limits) -> Result<String, FontError> {
    let cost = u64::try_from(name.len())
        .ok()
        .and_then(|n| n.checked_mul(6))
        .and_then(|n| n.checked_add(128))
        .ok_or_else(|| error(ErrorCode::LimitExceeded, "family fold length overflow"))?;
    if cost > limits.transient_bytes() {
        return Err(FontError::transient_limit(
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
    aliases: Vec<Arc<[String]>>,
}
impl FontCatalogBuilder {
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            records: Vec::new(),
            blobs: Vec::new(),
            bytes: 0,
            names: 0,
            aliases: Vec::new(),
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
        if length == 0 {
            return Err(error(ErrorCode::InvalidArgument, "font bytes are empty"));
        }
        if length > self.limits.font_bytes() {
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
        // Preflight every fold and the publication overlap before allocating
        // the alias array. Use the same expansion bound as fold_family.
        let alias_peak = metadata.aliases.iter().try_fold(0_u64, |sum, name| {
            (name.len() as u64)
                .checked_mul(6)
                .and_then(|n| n.checked_add(128 + 3 * std::mem::size_of::<String>() as u64))
                .and_then(|n| sum.checked_add(n))
                .ok_or_else(|| error(ErrorCode::LimitExceeded, "catalog aliases overflow"))
        })?;
        if intake_cost
            .checked_add(alias_peak)
            .is_none_or(|n| n > self.limits.transient_bytes())
        {
            return Err(error(
                ErrorCode::LimitExceeded,
                "catalog aliases transient budget exceeded",
            ));
        }
        let mut aliases = Vec::new();
        aliases
            .try_reserve_exact(metadata.aliases.len())
            .map_err(|_| {
                error(
                    ErrorCode::AllocationFailed,
                    "catalog aliases allocation failed",
                )
            })?;
        let mut alias_cost = 0_u64;
        for alias in metadata.aliases {
            let alias = fold_family(&alias, &self.limits)?;
            alias_cost = alias_cost
                .checked_add(alias.len() as u64)
                .and_then(|n| n.checked_add(3 * std::mem::size_of::<String>() as u64))
                .ok_or_else(|| error(ErrorCode::LimitExceeded, "catalog aliases overflow"))?;
            aliases.push(alias);
        }
        let names = self
            .names
            .checked_add(metadata.family.len() as u64)
            .and_then(|n| n.checked_add(folded.len() as u64))
            .and_then(|n| n.checked_add(alias_cost))
            .ok_or_else(|| error(ErrorCode::LimitExceeded, "catalog name size overflow"))?;
        if intake_cost
            .checked_add(alias_cost)
            .is_none_or(|n| n > self.limits.transient_bytes())
        {
            return Err(error(
                ErrorCode::LimitExceeded,
                "catalog aliases transient budget exceeded",
            ));
        }
        let record = FontRecord::from_unverified(
            FontBlob::from_unverified(id, data.clone()),
            Arc::from(metadata.family),
            Arc::from(folded),
            metadata.weight,
            metadata.italic,
        )?;
        self.records
            .try_reserve(1)
            .map_err(|_| error(ErrorCode::AllocationFailed, "catalog allocation failed"))?;
        self.aliases.try_reserve(1).map_err(|_| {
            error(
                ErrorCode::AllocationFailed,
                "catalog aliases allocation failed",
            )
        })?;
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
        self.aliases.push(aliases.into());
        self.bytes = new_total;
        self.names = names;
        Ok(id)
    }
    /// A renderer/session owns one ingestion owner across all catalog builders,
    /// so replacing a catalog cannot accidentally recycle a resolution identity.
    pub fn snapshot(
        &self,
        ingestion: &mut CatalogIngestor,
    ) -> Result<Arc<ValidatedFontCatalog>, FontError> {
        let mut records = Vec::new();
        records.try_reserve_exact(self.records.len()).map_err(|_| {
            error(
                ErrorCode::AllocationFailed,
                "catalog snapshot allocation failed",
            )
        })?;
        records.extend(self.records.iter().cloned());
        let mut aliases = Vec::new();
        aliases.try_reserve_exact(self.aliases.len()).map_err(|_| {
            error(
                ErrorCode::AllocationFailed,
                "catalog aliases snapshot allocation failed",
            )
        })?;
        aliases.extend(self.aliases.iter().cloned());
        // All builder records originate from register: digest, native face index,
        // family and attributes were verified, with collisions checked by bytes.
        Ok(Arc::new(ValidatedFontCatalog {
            inner: ingestion.publish_unverified(records.into())?,
            aliases: aliases.into(),
        }))
    }
}
/// Imports unverified shared descriptors. All fields are checked against freshly
/// derived native records; no claimed digest, family or revision is trusted.
/// Failure publishes no validated catalog and consumes no output revision.
pub fn validate_catalog(
    draft: &FontCatalog,
    context: &mut FontContext,
    ingestion: &mut CatalogIngestor,
    limits: Limits,
) -> Result<Arc<ValidatedFontCatalog>, FontError> {
    let mut builder = FontCatalogBuilder::new(limits);
    for claimed in draft.faces() {
        context.check_bytes(claimed.blob().bytes())?;
        check_blob_identity(claimed.blob())?;
        let previous = builder.records.len();
        let id = builder.register(
            context,
            claimed.blob().bytes(),
            claimed.blob().id().face_index(),
        )?;
        if builder.records.len() == previous {
            return Err(error(
                ErrorCode::InvalidArgument,
                "duplicate font identity in imported catalog",
            ));
        }
        let actual = &builder.records[previous];
        if id != claimed.blob().id()
            || actual.family() != claimed.family()
            || actual.folded_family() != claimed.folded_family()
            || actual.weight() != claimed.weight()
            || actual.italic() != claimed.italic()
        {
            return Err(error(
                ErrorCode::InvalidArgument,
                "unverified font metadata differs from native bytes",
            ));
        }
    }
    builder.snapshot(ingestion)
}
pub fn resolve_family(
    catalog: &ValidatedFontCatalog,
    request: &FontRequest,
    limits: &Limits,
) -> Result<FontId, FontError> {
    select_family_aliases(catalog.faces(), Some(&catalog.aliases), request, limits)
}
#[cfg(test)]
fn select_family(
    records: &[FontRecord],
    request: &FontRequest,
    limits: &Limits,
) -> Result<FontId, FontError> {
    select_family_aliases(records, None, request, limits)
}
fn select_family_aliases(
    records: &[FontRecord],
    aliases: Option<&[Arc<[String]>]>,
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
    records
        .iter()
        .enumerate()
        .filter(|(index, record)| {
            record.folded_family() == folded
                || aliases.is_some_and(|names| names[*index].contains(&folded))
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alias_membership_preserves_weight_italic_and_ordinal_ties() {
        let records: Vec<_> = [(300, false), (500, true), (500, false), (500, false)]
            .into_iter()
            .enumerate()
            .map(|(index, (weight, italic))| {
                FontRecord::from_unverified(
                    FontBlob::from_unverified(FontId::new([index as u8; 32], 0), Arc::from([1])),
                    Arc::from("Primary"),
                    Arc::from("primary"),
                    weight,
                    italic,
                )
                .unwrap()
            })
            .collect();
        let aliases: Vec<Arc<[String]>> = (0..4)
            .map(|_| Arc::from([String::from("別名"), String::from("alternate")]))
            .collect();
        for family in ["別名", "ALTERNATE"] {
            for (weight, italic, ordinal) in [(400, true, 1), (500, false, 2), (500, true, 1)] {
                let request = FontRequest::new(Arc::from(family), weight, italic, false);
                let result =
                    select_family_aliases(&records, Some(&aliases), &request, &Limits::default())
                        .unwrap();
                assert_eq!(result.sha256(), &[ordinal; 32]);
            }
        }
    }
    #[test]
    fn catalog_ties_follow_weight_then_italic_then_registration_order_synthetic() {
        // Synthetic descriptors only reach the private selector. No native
        // validation marker, fixture feature or unchecked runtime constructor.
        let bytes: Arc<[u8]> = Arc::from([1]);
        let records = [
            (0, 300, false),
            (1, 500, true),
            (2, 500, false),
            (3, 500, false),
        ]
        .into_iter()
        .map(|(ordinal, weight, italic)| {
            FontRecord::from_unverified(
                FontBlob::from_unverified(FontId::new([ordinal; 32], 0), bytes.clone()),
                Arc::from("Family"),
                Arc::from("family"),
                weight,
                italic,
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
        let request = |family: &str, weight, italic| {
            FontRequest::new(Arc::from(family), weight, italic, false)
        };
        let limits = Limits::default();
        assert_eq!(
            select_family(&records, &request("Family", 400, true), &limits)
                .unwrap()
                .sha256(),
            &[1; 32]
        );
        assert_eq!(
            select_family(&records, &request("Family", 500, false), &limits)
                .unwrap()
                .sha256(),
            &[2; 32]
        );
        assert_eq!(
            select_family(&records, &request("absent", 400, false), &limits)
                .unwrap_err()
                .code(),
            ErrorCode::FontUnavailable
        );
    }
}
