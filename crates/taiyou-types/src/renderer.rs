//! C7 single-worker configuration and checked square-pixel viewport.
use crate::*;
use std::num::NonZeroUsize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CacheConfig {
    Disabled,
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
            limits: Limits::default().with_cache_bytes(0),
            cache: CacheConfig::Disabled,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Viewport {
    size: Size,
    content: PixelRect,
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
        Ok(Self { size, content })
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
}
#[derive(Clone, Copy, Debug)]
pub struct RenderRequest {
    pub now: TimeUs,
    pub viewport: Viewport,
    pub unsupported: UnsupportedPolicy,
}
