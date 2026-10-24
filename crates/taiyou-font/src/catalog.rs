use crate::native::FontContext;
use icu_casemap::CaseMapper;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use taiyou_types::*;
#[derive(Clone, Copy, Debug)]
pub(crate) struct VerticalMetrics {
    pub cff: bool,
    pub typo_descent: i16,
    pub ascent: u16,
    pub descent: u16,
}
/// A bounded Windows host observation for one complete missing cluster and
/// requested cell size. Explicit catalogs never contain these observations.
#[derive(Clone, Debug)]
pub(crate) struct HostLink {
    pub primary: FontId,
    pub fallback: FontId,
    pub text: Arc<str>,
    pub size: FiniteF64,
    pub scale: FiniteF64,
    pub request: FontRequest,
    pub baseline_delta: FiniteF64,
    pub advance: FiniteF64,
}
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
    cells: Arc<[(u16, u16, u16)]>,
    verticals: Arc<[VerticalMetrics]>,
    legacy_charmaps: Arc<[Option<u16>]>,
    host_links: Arc<[HostLink]>,
}
impl ValidatedFontCatalog {
    pub fn has_host_font_links(&self) -> bool { !self.host_links.is_empty() }
    pub(crate) fn host_link(&self, primary: FontId, fallback: FontId, text: &str, size: FiniteF64, request: &FontRequest) -> Option<&HostLink> {
        self.host_links.iter().find(|link| link.primary == primary && link.fallback == fallback && link.text.as_ref() == text && link.size == size && &link.request == request)
    }
    /// Resolve the already admitted host observation; performs no OS discovery.
    /// Geometry consumes the same immutable scale selected during shaping.
    pub fn admitted_font_scale(&self, primary: FontId, fallback: FontId, text: &str, size: FiniteF64, request: &FontRequest, vertical: bool) -> Result<FiniteF64, FontError> {
        if !vertical {
            if let Some(link) = self.host_link(primary, fallback, text, size, request) {
                return Ok(link.scale);
            }
        }
        self.ass_oriented_font_scale(fallback, size, vertical)
    }
    pub fn admitted_metric_font(&self, primary: FontId, fallback: FontId, text: &str, size: FiniteF64, request: &FontRequest, vertical: bool) -> FontId {
        if !vertical && self.host_link(primary, fallback, text, size, request).is_some() { primary } else { fallback }
    }
    pub fn needs_host_font_link(&self, primary: FontId, fallback: FontId, text: &str, size: FiniteF64, request: &FontRequest, vertical: bool) -> bool {
        !vertical && primary != fallback && self.host_links.iter().any(|link| link.fallback == fallback)
            && self.host_link(primary, fallback, text, size, request).is_none()
    }
    pub fn admitted_baseline_delta(&self, primary: FontId, fallback: FontId, text: &str, size: FiniteF64, request: &FontRequest, vertical: bool) -> FiniteF64 {
        if vertical { FiniteF64::ZERO } else { self.host_link(primary, fallback, text, size, request).map_or(FiniteF64::ZERO, |link| link.baseline_delta) }
    }
    pub fn ass_oriented_cell_metrics(
        &self,
        font: FontId,
        vertical: bool,
    ) -> Result<(u16, u16, u16), FontError> {
        let horizontal = self.ass_cell_metrics(font)?;
        if !vertical {
            return Ok(horizontal);
        }
        let index = self
            .faces()
            .iter()
            .position(|f| f.blob().id() == font)
            .ok_or_else(|| error(ErrorCode::InvalidArgument, "font absent"))?;
        let v = self.verticals[index];
        Ok((horizontal.0, v.ascent, v.descent))
    }
    /// Validated face's design-unit decoration centres and thicknesses.
    pub fn decoration_metrics(
        &self,
        context: &mut FontContext,
        font: FontId,
    ) -> Result<[(i16, i16); 2], FontError> {
        let face = self
            .faces()
            .iter()
            .find(|f| f.blob().id() == font)
            .ok_or_else(|| error(ErrorCode::FontUnavailable, "decoration face absent"))?;
        context.decoration_metrics_authenticated(face.blob())
    }
    /// Validated (UPEM, Windows cell ascent, Windows cell descent) of one face.
    pub fn ass_cell_metrics(&self, font: FontId) -> Result<(u16, u16, u16), FontError> {
        let index = self
            .faces()
            .iter()
            .position(|f| f.blob().id() == font)
            .ok_or_else(|| {
                error(
                    ErrorCode::InvalidArgument,
                    "font absent from validated catalog",
                )
            })?;
        Ok(self.cells[index])
    }
    /// Positive GDI cell-height sizing at xy's 64-unit font construction scale.
    pub fn ass_font_scale(&self, font: FontId, size: FiniteF64) -> Result<FiniteF64, FontError> {
        self.ass_oriented_font_scale(font, size, false)
    }
    pub fn ass_oriented_font_scale(
        &self,
        font: FontId,
        size: FiniteF64,
        vertical: bool,
    ) -> Result<FiniteF64, FontError> {
        if size.get() <= 0.0 {
            return Err(error(
                ErrorCode::InvalidArgument,
                "ASS cell height must be positive",
            ));
        }
        let (upem, ascent, descent) = self.ass_oriented_cell_metrics(font, vertical)?;
        let height = (size.get() * 64.0 + 0.5).floor();
        if !height.is_finite() || height > f64::from(i32::MAX) {
            return Err(error(
                ErrorCode::LimitExceeded,
                "ASS cell height exceeds GDI coordinate range",
            ));
        }
        let em =
            (height * f64::from(upem) / (f64::from(ascent) + f64::from(descent)) + 0.5).floor();
        if em <= 0.0 {
            return Err(error(
                ErrorCode::UnsupportedFeature,
                "ASS cell rounds to an unspecified GDI default height",
            ));
        }
        FiniteF64::new(em / (64.0 * f64::from(upem))).map_err(|e| error(e.code(), e.detail()))
    }
    pub(crate) fn ass_oriented_line_metrics(
        &self,
        font: FontId,
        size: FiniteF64,
        vertical: bool,
    ) -> Result<RunMetrics, FontError> {
        let (upem, ascent, descent) = self.ass_oriented_cell_metrics(font, vertical)?;
        let scale = self.ass_oriented_font_scale(font, size, vertical)?.get();
        let metric = |v: u16| {
            let native = (f64::from(v) * scale * 64.0 + 0.5).floor();
            FiniteF64::new(((native + 4.0) / 8.0).floor() / 8.0)
                .map_err(|e| error(e.code(), e.detail()))
        };
        RunMetrics::new(metric(ascent)?, metric(descent)?, FiniteF64::ZERO, upem)
    }
    pub(crate) fn ass_oriented_baseline_offset(
        &self,
        font: FontId,
        size: FiniteF64,
        vertical: bool,
    ) -> Result<f64, FontError> {
        let (_, ascent, _) = self.ass_oriented_cell_metrics(font, vertical)?;
        let scale = self.ass_oriented_font_scale(font, size, vertical)?.get();
        let native_ascent = (f64::from(ascent) * scale * 64.0 + 0.5).floor() / 64.0;
        // GDI TextOut's TA_TOP path contains native tmAscent. CLine::PaintAll
        // aligns word tops using the separately rounded eighth-unit ascent.
        Ok(native_ascent
            - self
                .ass_oriented_line_metrics(font, size, vertical)?
                .ascent()
                .get())
    }
    pub fn revision(&self) -> CatalogRevision {
        self.inner.revision()
    }
    pub fn faces(&self) -> &[FontRecord] {
        self.inner.faces()
    }
    /// Synthetic slant for an italic request resolved to a nonitalic face.
    /// Real italic faces already carry their own outlines and are not sheared.
    pub fn synthetic_italic_shear(
        &self,
        font: FontId,
        requested: bool,
    ) -> Result<FiniteF64, FontError> {
        let index = self
            .faces()
            .iter()
            .position(|face| face.blob().id() == font)
            .ok_or_else(|| error(ErrorCode::InvalidArgument, "font absent"))?;
        let fixed = if requested && !self.faces()[index].italic() {
            if self.verticals[index].cff {
                0x2D24
            } else {
                0x5700
            }
        } else {
            0
        };
        FiniteF64::new(f64::from(fixed) / 65536.0).map_err(|e| error(e.code(), e.detail()))
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
        let vertical = if request.vertical {
            Some(
                self.verticals[self
                    .faces()
                    .iter()
                    .position(|f| f.blob().id() == font)
                    .ok_or_else(|| error(ErrorCode::InvalidArgument, "font absent"))?],
            )
        } else {
            None
        };
        let index = self
            .faces()
            .iter()
            .position(|face| face.blob().id() == font)
            .ok_or_else(|| error(ErrorCode::InvalidArgument, "font absent"))?;
        context.shape_authenticated_with_ink(
            face.blob(),
            request,
            limits,
            collect_ink,
            crate::native::NativeFaceOptions {
                vertical,
                legacy_charmap: self.legacy_charmaps[index],
            },
        )
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
    cells: Vec<(u16, u16, u16)>,
    verticals: Vec<VerticalMetrics>,
    legacy_charmaps: Vec<Option<u16>>,
    host_links: Vec<HostLink>,
}
impl FontCatalogBuilder {
    /// Re-admit a validated catalog under this owner's limits, preserving host
    /// substitutions when a renderer appends embedded fonts. The old snapshot
    /// remains immutable; native bytes are authenticated normally.
    pub fn from_catalog(
        context: &mut FontContext,
        catalog: &ValidatedFontCatalog,
        limits: Limits,
    ) -> Result<Self, FontError> {
        let mut builder = Self::new(limits);
        for face in catalog.faces() {
            builder.register(context, face.blob().bytes(), face.blob().id().face_index())?;
        }
        for (index, aliases) in catalog.aliases.iter().enumerate() {
            for alias in aliases.iter() {
                let face = &catalog.faces()[index];
                builder.host_substitution(
                    &FontRequest::new(
                        Arc::from(alias.as_str()),
                        face.weight(),
                        face.italic(),
                        false,
                    ),
                    face.blob().id(),
                )?;
            }
        }
        for link in catalog.host_links.iter() {
            builder.host_link(link.clone())?;
        }
        Ok(builder)
    }
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            records: Vec::new(),
            blobs: Vec::new(),
            bytes: 0,
            names: 0,
            aliases: Vec::new(),
            cells: Vec::new(),
            verticals: Vec::new(),
            legacy_charmaps: Vec::new(),
            host_links: Vec::new(),
        }
    }
    pub const fn registered_bytes(&self) -> u64 {
        self.bytes
    }
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) fn host_link(&mut self, link: HostLink) -> Result<(), FontError> {
        if let Some(old) = self.host_links.iter().find(|old| old.primary == link.primary && old.fallback == link.fallback && old.size == link.size && old.text == link.text && old.request == link.request) {
            if old.scale != link.scale || old.baseline_delta != link.baseline_delta || old.advance != link.advance {
                return Err(error(ErrorCode::NativeFailure, "inconsistent Windows font-link observation"));
            }
            return Ok(());
        }
        let charge = (self.host_links.len() as u64 + 1).checked_mul(3 * std::mem::size_of::<HostLink>() as u64)
            .and_then(|n| n.checked_add((link.text.len() as u64 + link.request.family().len() as u64) * 3))
            .and_then(|n| n.checked_add(self.names)).ok_or_else(|| FontError::transient_limit("host links overflow"))?;
        if charge > self.limits.transient_bytes() {
            return Err(FontError::transient_limit("host links exceed budget"));
        }
        self.host_links.try_reserve_exact(1).map_err(|_| error(ErrorCode::AllocationFailed, "host link allocation"))?;
        self.names = self.names.checked_add((link.text.len() as u64 + link.request.family().len() as u64) * 3).ok_or_else(|| FontError::transient_limit("host link names overflow"))?;
        self.host_links.push(link);
        Ok(())
    }
    /// Record a host provider's substitution only when no explicitly admitted
    /// face already matches the requested family. This is catalog data, never
    /// an implicit system lookup during resolution or shaping.
    pub(crate) fn host_substitution(
        &mut self,
        request: &FontRequest,
        id: FontId,
    ) -> Result<(), FontError> {
        let name = fold_family(
            request
                .family()
                .strip_prefix('@')
                .unwrap_or(request.family()),
            &self.limits,
        )?;
        if self
            .records
            .iter()
            .enumerate()
            .any(|(i, r)| r.folded_family() == name || self.aliases[i].contains(&name))
        {
            return Ok(());
        }
        let index = self
            .records
            .iter()
            .position(|r| r.blob().id() == id)
            .ok_or_else(|| {
                error(
                    ErrorCode::InvalidArgument,
                    "host substitution face is absent",
                )
            })?;
        let charge = (name.len() as u64)
            .checked_add(3 * std::mem::size_of::<String>() as u64)
            .and_then(|n| n.checked_add(self.names))
            .ok_or_else(|| error(ErrorCode::LimitExceeded, "host substitution name overflow"))?;
        if charge
            .checked_mul(3)
            .is_none_or(|n| n > self.limits.transient_bytes())
        {
            return Err(FontError::transient_limit(
                "host substitution names exceed budget",
            ));
        }
        let mut aliases = Vec::new();
        aliases
            .try_reserve_exact(self.aliases[index].len() + 1)
            .map_err(|_| {
                error(
                    ErrorCode::AllocationFailed,
                    "host substitution allocation failed",
                )
            })?;
        aliases.extend(self.aliases[index].iter().cloned());
        aliases.push(name);
        self.aliases[index] = aliases.into();
        self.names = charge;
        Ok(())
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
            .and_then(|n| {
                n.checked_mul(
                    3 * (std::mem::size_of::<FontRecord>()
                        + std::mem::size_of::<(u16, u16, u16)>()
                        + std::mem::size_of::<VerticalMetrics>()
                        + std::mem::size_of::<Option<u16>>()) as u64,
                )
            })
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
        self.cells.try_reserve(1).map_err(|_| {
            error(
                ErrorCode::AllocationFailed,
                "catalog cell metric allocation failed",
            )
        })?;
        self.verticals.try_reserve(1).map_err(|_| {
            error(
                ErrorCode::AllocationFailed,
                "vertical metric allocation failed",
            )
        })?;
        self.legacy_charmaps.try_reserve(1).map_err(|_| {
            error(
                ErrorCode::AllocationFailed,
                "catalog cmap selector allocation failed",
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
        self.cells.push(metadata.cell);
        self.verticals.push(metadata.vertical);
        self.legacy_charmaps.push(metadata.legacy_charmap);
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
        let mut cells = Vec::new();
        cells.try_reserve_exact(self.cells.len()).map_err(|_| {
            error(
                ErrorCode::AllocationFailed,
                "catalog cell metric snapshot allocation failed",
            )
        })?;
        cells.extend_from_slice(&self.cells);
        let mut verticals = Vec::new();
        verticals
            .try_reserve_exact(self.verticals.len())
            .map_err(|_| {
                error(
                    ErrorCode::AllocationFailed,
                    "vertical metric snapshot allocation failed",
                )
            })?;
        verticals.extend_from_slice(&self.verticals);
        let mut legacy_charmaps = Vec::new();
        legacy_charmaps
            .try_reserve_exact(self.legacy_charmaps.len())
            .map_err(|_| {
                error(
                    ErrorCode::AllocationFailed,
                    "catalog cmap selector snapshot allocation failed",
                )
            })?;
        legacy_charmaps.extend_from_slice(&self.legacy_charmaps);
        let mut host_links = Vec::new();
        host_links.try_reserve_exact(self.host_links.len()).map_err(|_| error(ErrorCode::AllocationFailed, "host links snapshot allocation"))?;
        host_links.extend(self.host_links.iter().cloned());
        // All builder records originate from register: digest, native face index,
        // family and attributes were verified, with collisions checked by bytes.
        Ok(Arc::new(ValidatedFontCatalog {
            inner: ingestion.publish_unverified(records.into())?,
            legacy_charmaps: legacy_charmaps.into(),
            aliases: aliases.into(),
            cells: cells.into(),
            verticals: verticals.into(),
            host_links: host_links.into(),
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
    let folded = fold_family(
        request
            .family()
            .strip_prefix('@')
            .unwrap_or(request.family()),
        limits,
    )?;
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
    fn host_link_observations_are_cluster_size_scoped_and_survive_rebuilding() {
        let limits = Limits::default();
        let mut context = FontContext::new().unwrap();
        let mut builder = FontCatalogBuilder::new(limits.clone());
        let primary = builder.register(&mut context, &std::fs::read(std::env::var("TAIYOU_TEST_FONT").unwrap()).unwrap(), 0).unwrap();
        let fallback = builder.register(&mut context, &std::fs::read(std::env::var("TAIYOU_TEST_BOLD").unwrap()).unwrap(), 0).unwrap();
        let mut ingestion = CatalogIngestor::new();
        let before = builder.snapshot(&mut ingestion).unwrap();
        let size = FiniteF64::new(60.0).unwrap();
        let scale = FiniteF64::new(0.02).unwrap();
        let request = FontRequest::new(Arc::from("Primary"), 400, false, false);
        let link = HostLink { primary, fallback, text: Arc::from("a\u{301}"), size, scale, request: request.clone(), baseline_delta: FiniteF64::new(0.015625).unwrap(), advance: FiniteF64::new(18.5).unwrap() };
        builder.host_link(link.clone()).unwrap();
        builder.host_link(link.clone()).unwrap();
        let after = builder.snapshot(&mut ingestion).unwrap();
        let copy = FontCatalogBuilder::from_catalog(&mut context, &after, limits).unwrap().snapshot(&mut ingestion).unwrap();
        assert!(!before.has_host_font_links());
        for catalog in [&after, &copy] {
            assert_eq!(catalog.host_links.len(), 1);
            assert_eq!(catalog.admitted_font_scale(primary, fallback, "a\u{301}", size, &request, false).unwrap(), scale);
            assert_eq!(catalog.admitted_metric_font(primary, fallback, "a\u{301}", size, &request, false), primary);
            assert_eq!(catalog.admitted_baseline_delta(primary, fallback, "a\u{301}", size, &request, false), link.baseline_delta);
            assert_eq!(catalog.host_link(primary, fallback, "a\u{301}", size, &request).unwrap().advance, link.advance);
            for (text, size, vertical) in [("a", size, false), ("a\u{301}", FiniteF64::new(61.0).unwrap(), false), ("a\u{301}", size, true)] {
                assert_eq!(catalog.admitted_font_scale(primary, fallback, text, size, &request, vertical).unwrap(), catalog.ass_oriented_font_scale(fallback, size, vertical).unwrap());
                assert_eq!(catalog.admitted_metric_font(primary, fallback, text, size, &request, vertical), fallback);
            }
            let other_style = FontRequest::new(Arc::from("Primary"), 700, true, false);
            assert_eq!(catalog.admitted_font_scale(primary, fallback, "a\u{301}", size, &other_style, false).unwrap(), catalog.ass_font_scale(fallback, size).unwrap());
            assert!(catalog.needs_host_font_link(primary, fallback, "a\u{301}", size, &other_style, false));
        }
        let mut inconsistent = link.clone();
        inconsistent.scale = FiniteF64::ONE;
        assert_eq!(builder.host_link(inconsistent).unwrap_err().code(), ErrorCode::NativeFailure);
        let mut inconsistent = link.clone();
        inconsistent.advance = FiniteF64::ONE;
        assert_eq!(builder.host_link(inconsistent).unwrap_err().code(), ErrorCode::NativeFailure);
        builder.limits = Limits::default().with_transient_bytes(1).unwrap();
        let mut another = link;
        another.text = Arc::from("b\u{301}");
        assert_eq!(builder.host_link(another).unwrap_err().code(), ErrorCode::LimitExceeded);
        assert_eq!(before.admitted_font_scale(primary, fallback, "a\u{301}", size, &request, false).unwrap(), before.ass_oriented_font_scale(fallback, size, false).unwrap());
    }
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
