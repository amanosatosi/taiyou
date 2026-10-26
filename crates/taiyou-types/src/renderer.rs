//! C7 explicit worker/cache configuration and checked square-pixel viewport.
use crate::*;
use std::num::NonZeroUsize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CacheConfig {
    Disabled,
    /// Retain raster masks, bypassing filtered-result retention for diagnostics.
    MasksOnly,
    /// Bounded intermediate retention within Limits::cache_bytes().
    Bounded,
}
#[derive(Clone, Debug)]
pub struct RendererConfig {
    pub workers: NonZeroUsize,
    pub profile: Profile,
    pub limits: Limits,
    pub cache: CacheConfig,
}
impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            workers: NonZeroUsize::MIN,
            profile: Profile::CoreV1,
            limits: Limits::default(),
            cache: CacheConfig::Bounded,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Viewport {
    size: Size,
    content: PixelRect,
    original: Option<Size>,
}
impl Viewport {
    /// Initial Core accepts square pixels only; content is contained in size.
    pub fn new(size: Size, content: PixelRect) -> Result<Self, RenderError> {
        if size.is_empty()
            || size.width() > i32::MAX as u32
            || size.height() > i32::MAX as u32
            || content.is_empty()
            || content.x0() < 0
            || content.y0() < 0
            || i64::from(content.x1()) > i64::from(size.width())
            || i64::from(content.y1()) > i64::from(size.height())
        {
            return Err(RenderError::new(
                ErrorCode::InvalidArgument,
                "invalid viewport/content rectangle",
            ));
        }
        Ok(Self {
            size,
            content,
            original: None,
        })
    }
    pub fn full(size: Size) -> Result<Self, RenderError> {
        let narrow = |n| {
            i32::try_from(n).map_err(|_| {
                RenderError::new(ErrorCode::InvalidArgument, "viewport coordinate overflow")
            })
        };
        let content = PixelRect::new(0, 0, narrow(size.width())?, narrow(size.height())?)
            .map_err(|e| RenderError::new(e.code(), e.detail()))?;
        Self::new(size, content)
    }
    pub const fn size(self) -> Size {
        self.size
    }
    pub const fn content(self) -> PixelRect {
        self.content
    }
    /// Original video dimensions are separate from a host's oversampled output.
    pub fn with_original_size(mut self, original: Size) -> Result<Self, RenderError> {
        if original.is_empty()
            || original.width() > i32::MAX as u32
            || original.height() > i32::MAX as u32
        {
            return Err(RenderError::new(
                ErrorCode::InvalidArgument,
                "invalid original video dimensions",
            ));
        }
        self.original = Some(original);
        Ok(self)
    }
    pub fn target_scale(self) -> [f64; 2] {
        self.original.map_or([1.0, 1.0], |s| {
            [
                f64::from(self.content.width()) / f64::from(s.width()),
                f64::from(self.content.height()) / f64::from(s.height()),
            ]
        })
    }
    pub fn original_size(self) -> Size {
        self.original
            .unwrap_or(Size::new(self.content.width(), self.content.height()))
    }
}
#[derive(Clone, Copy, Debug)]
pub struct RenderRequest {
    pub now: TimeUs,
    pub viewport: Viewport,
    pub unsupported: UnsupportedPolicy,
}
