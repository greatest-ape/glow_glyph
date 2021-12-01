//! A fast text renderer for [`glow`]. Powered by [`glyph_brush`].
//!
//! [`glow`]: https://github.com/grovesNL/glow
//! [`glyph_brush`]: https://github.com/alexheretic/glyph-brush/tree/master/glyph-brush
#![deny(unused_results)]
mod builder;
mod pipeline;
mod region;

use pipeline::{compatibility, core};
pub use region::Region;

pub use builder::GlyphBrushBuilder;
pub use glyph_brush::ab_glyph;
pub use glyph_brush::{
    BuiltInLineBreaker, Extra, FontId, GlyphCruncher, GlyphPositioner,
    HorizontalAlign, Layout, LineBreak, LineBreaker, Section, SectionGeometry,
    SectionGlyph, SectionGlyphIter, SectionText, Text, VerticalAlign,
};

use ab_glyph::{Font, FontArc, Rect};

use ::core::hash::BuildHasher;
use std::borrow::Cow;

use glyph_brush::{BrushAction, BrushError, DefaultSectionHasher};
use log::{log_enabled, warn};

/// Object allowing glyph drawing, containing cache state. Manages glyph positioning cacheing,
/// glyph draw caching & efficient GPU texture cache updating and re-sizing on demand.
///
/// Build using a [`GlyphBrushBuilder`](struct.GlyphBrushBuilder.html).
pub enum GlyphBrush<F = FontArc, H = DefaultSectionHasher> {
    Core {
        pipeline: core::Pipeline,
        glyph_brush: glyph_brush::GlyphBrush<core::Instance, Extra, F, H>,
    },
    Compatibility {
        pipeline: compatibility::Pipeline,
        glyph_brush:
            glyph_brush::GlyphBrush<[compatibility::Vertex; 4], Extra, F, H>,
    },
}

impl<F: Font, H: BuildHasher> GlyphBrush<F, H> {
    /// Queues a section/layout to be drawn by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be
    /// called multiple times to queue multiple sections for drawing.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn queue<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, Section<'a>>>,
    {
        match self {
            GlyphBrush::Compatibility { glyph_brush, .. } => {
                glyph_brush.queue(section)
            }
            GlyphBrush::Core { glyph_brush, .. } => glyph_brush.queue(section),
        }
    }

    /// Queues a section/layout to be drawn by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be
    /// called multiple times to queue multiple sections for drawing.
    ///
    /// Used to provide custom `GlyphPositioner` logic, if using built-in
    /// [`Layout`](enum.Layout.html) simply use
    /// [`queue`](struct.GlyphBrush.html#method.queue)
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn queue_custom_layout<'a, S, G>(
        &mut self,
        section: S,
        custom_layout: &G,
    ) where
        G: GlyphPositioner,
        S: Into<Cow<'a, Section<'a>>>,
    {
        match self {
            GlyphBrush::Compatibility { glyph_brush, .. } => {
                glyph_brush.queue_custom_layout(section, custom_layout)
            }
            GlyphBrush::Core { glyph_brush, .. } => {
                glyph_brush.queue_custom_layout(section, custom_layout)
            }
        }
    }

    /// Queues pre-positioned glyphs to be processed by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be
    /// called multiple times.
    #[inline]
    pub fn queue_pre_positioned(
        &mut self,
        glyphs: Vec<SectionGlyph>,
        extra: Vec<Extra>,
        bounds: Rect,
    ) {
        match self {
            GlyphBrush::Compatibility { glyph_brush, .. } => {
                glyph_brush.queue_pre_positioned(glyphs, extra, bounds)
            }
            GlyphBrush::Core { glyph_brush, .. } => {
                glyph_brush.queue_pre_positioned(glyphs, extra, bounds)
            }
        }
    }

    /// Retains the section in the cache as if it had been used in the last
    /// draw-frame.
    ///
    /// Should not be necessary unless using multiple draws per frame with
    /// distinct transforms, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn keep_cached_custom_layout<'a, S, G>(
        &mut self,
        section: S,
        custom_layout: &G,
    ) where
        S: Into<Cow<'a, Section<'a>>>,
        G: GlyphPositioner,
    {
        match self {
            GlyphBrush::Compatibility { glyph_brush, .. } => {
                glyph_brush.keep_cached_custom_layout(section, custom_layout)
            }
            GlyphBrush::Core { glyph_brush, .. } => {
                glyph_brush.keep_cached_custom_layout(section, custom_layout)
            }
        }
    }

    /// Retains the section in the cache as if it had been used in the last
    /// draw-frame.
    ///
    /// Should not be necessary unless using multiple draws per frame with
    /// distinct transforms, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn keep_cached<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, Section<'a>>>,
    {
        match self {
            GlyphBrush::Compatibility { glyph_brush, .. } => {
                glyph_brush.keep_cached(section)
            }
            GlyphBrush::Core { glyph_brush, .. } => {
                glyph_brush.keep_cached(section)
            }
        }
    }

    /// Returns the available fonts.
    ///
    /// The `FontId` corresponds to the index of the font data.
    #[inline]
    pub fn fonts(&self) -> &[F] {
        match self {
            GlyphBrush::Compatibility { glyph_brush, .. } => {
                glyph_brush.fonts()
            }
            GlyphBrush::Core { glyph_brush, .. } => glyph_brush.fonts(),
        }
    }

    /// Adds an additional font to the one(s) initially added on build.
    ///
    /// Returns a new [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font(&mut self, font: F) -> FontId {
        match self {
            GlyphBrush::Compatibility { glyph_brush, .. } => {
                glyph_brush.add_font(font)
            }
            GlyphBrush::Core { glyph_brush, .. } => glyph_brush.add_font(font),
        }
    }
}

impl<F: Font + Sync, H: BuildHasher> GlyphBrush<F, H> {
    /// Draws all queued sections onto a render target.
    /// See [`queue`](struct.GlyphBrush.html#method.queue).
    ///
    /// Trims the cache, see [caching behaviour](#caching-behaviour).
    ///
    /// # Panics
    /// Panics if the provided `target` has a texture format that does not match
    /// the `render_format` provided on creation of the `GlyphBrush`.
    #[inline]
    pub fn draw_queued(
        &mut self,
        context: &glow::Context,
        target_width: u32,
        target_height: u32,
    ) -> Result<(), String> {
        self.draw_queued_with_transform(
            context,
            orthographic_projection(target_width, target_height),
        )
    }

    /// Draws all queued sections onto a render target, applying a position
    /// transform (e.g. a projection).
    /// See [`queue`](struct.GlyphBrush.html#method.queue).
    ///
    /// Trims the cache, see [caching behaviour](#caching-behaviour).
    ///
    /// # Panics
    /// Panics if the provided `target` has a texture format that does not match
    /// the `render_format` provided on creation of the `GlyphBrush`.
    #[inline]
    pub fn draw_queued_with_transform(
        &mut self,
        context: &glow::Context,
        transform: [f32; 16],
    ) -> Result<(), String> {
        self.process_queued(context);

        match self {
            GlyphBrush::Compatibility { pipeline, .. } => {
                pipeline.draw(context, transform, None);
            }
            GlyphBrush::Core { pipeline, .. } => {
                pipeline.draw(context, transform, None);
            }
        }

        Ok(())
    }

    /// Draws all queued sections onto a render target, applying a position
    /// transform (e.g. a projection) and a scissoring region.
    /// See [`queue`](struct.GlyphBrush.html#method.queue).
    ///
    /// Trims the cache, see [caching behaviour](#caching-behaviour).
    ///
    /// # Panics
    /// Panics if the provided `target` has a texture format that does not match
    /// the `render_format` provided on creation of the `GlyphBrush`.
    #[inline]
    pub fn draw_queued_with_transform_and_scissoring(
        &mut self,
        context: &glow::Context,
        transform: [f32; 16],
        region: Region,
    ) -> Result<(), String> {
        self.process_queued(context);

        match self {
            GlyphBrush::Compatibility { pipeline, .. } => {
                pipeline.draw(context, transform, Some(region));
            }
            GlyphBrush::Core { pipeline, .. } => {
                pipeline.draw(context, transform, Some(region));
            }
        }

        Ok(())
    }

    fn process_queued(&mut self, context: &glow::Context) {
        match self {
            GlyphBrush::Compatibility {
                glyph_brush,
                pipeline,
            } => {
                let mut brush_action;

                loop {
                    brush_action = glyph_brush.process_queued(
                        |rect, tex_data| {
                            let offset =
                                [rect.min[0] as u16, rect.min[1] as u16];
                            let size =
                                [rect.width() as u16, rect.height() as u16];

                            pipeline
                                .update_cache(context, offset, size, tex_data);
                        },
                        |glyph| compatibility::Vertex::from_vertex(&glyph),
                    );

                    match brush_action {
                        Ok(_) => break,
                        Err(BrushError::TextureTooSmall { suggested }) => {
                            let max_image_size =
                                pipeline.get_max_texture_size();

                            let (new_width, new_height) = if (suggested.0
                                > max_image_size
                                || suggested.1 > max_image_size)
                                && (glyph_brush.texture_dimensions().0
                                    < max_image_size
                                    || glyph_brush.texture_dimensions().1
                                        < max_image_size)
                            {
                                (max_image_size, max_image_size)
                            } else {
                                suggested
                            };

                            if log_enabled!(log::Level::Warn) {
                                warn!(
                            "Increasing glyph texture size {old:?} -> {new:?}. \
                             Consider building with `.initial_cache_size({new:?})` to avoid \
                             resizing",
                            old = glyph_brush.texture_dimensions(),
                            new = (new_width, new_height),
                        );
                            }

                            pipeline.increase_cache_size(
                                context, new_width, new_height,
                            );
                            glyph_brush.resize_texture(new_width, new_height);
                        }
                    }
                }

                match brush_action.unwrap() {
                    BrushAction::Draw(verts) => {
                        pipeline.upload(context, &verts);
                    }
                    BrushAction::ReDraw => {}
                };
            }
            GlyphBrush::Core {
                glyph_brush,
                pipeline,
            } => {
                let mut brush_action;

                loop {
                    brush_action = glyph_brush.process_queued(
                        |rect, tex_data| {
                            let offset =
                                [rect.min[0] as u16, rect.min[1] as u16];
                            let size =
                                [rect.width() as u16, rect.height() as u16];

                            pipeline
                                .update_cache(context, offset, size, tex_data);
                        },
                        core::Instance::from_vertex,
                    );

                    match brush_action {
                        Ok(_) => break,
                        Err(BrushError::TextureTooSmall { suggested }) => {
                            let max_image_size =
                                pipeline.get_max_texture_size();

                            let (new_width, new_height) = if (suggested.0
                                > max_image_size
                                || suggested.1 > max_image_size)
                                && (glyph_brush.texture_dimensions().0
                                    < max_image_size
                                    || glyph_brush.texture_dimensions().1
                                        < max_image_size)
                            {
                                (max_image_size, max_image_size)
                            } else {
                                suggested
                            };

                            if log_enabled!(log::Level::Warn) {
                                warn!(
                            "Increasing glyph texture size {old:?} -> {new:?}. \
                             Consider building with `.initial_cache_size({new:?})` to avoid \
                             resizing",
                            old = glyph_brush.texture_dimensions(),
                            new = (new_width, new_height),
                        );
                            }

                            pipeline.increase_cache_size(
                                context, new_width, new_height,
                            );
                            glyph_brush.resize_texture(new_width, new_height);
                        }
                    }
                }

                match brush_action.unwrap() {
                    BrushAction::Draw(verts) => {
                        pipeline.upload(context, &verts);
                    }
                    BrushAction::ReDraw => {}
                };
            }
        }
    }
}

impl<F: Font, H: BuildHasher> GlyphBrush<F, H> {
    fn new(
        gl: &glow::Context,
        raw_builder: glyph_brush::GlyphBrushBuilder<F, H>,
    ) -> Self {
        use glow::HasContext;

        let version = gl.version();

        if version.major >= 3 {
            log::info!("Mode: core");

            let glyph_brush = raw_builder.build();
            let (cache_width, cache_height) = glyph_brush.texture_dimensions();

            GlyphBrush::Core {
                pipeline: core::Pipeline::new(gl, cache_width, cache_height),
                glyph_brush,
            }
        } else {
            log::info!("Mode: compatibility");

            let glyph_brush = raw_builder.build();
            let (cache_width, cache_height) = glyph_brush.texture_dimensions();

            GlyphBrush::Compatibility {
                pipeline: compatibility::Pipeline::new(
                    gl,
                    cache_width,
                    cache_height,
                ),
                glyph_brush,
            }
        }
    }
}

/// Helper function to generate a generate a transform matrix.
pub fn orthographic_projection(width: u32, height: u32) -> [f32; 16] {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    [
        2.0 / width as f32, 0.0, 0.0, 0.0,
        0.0, -2.0 / height as f32, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        -1.0, 1.0, 0.0, 1.0,
    ]
}

impl<F: Font, H: BuildHasher> GlyphCruncher<F> for GlyphBrush<F, H> {
    #[inline]
    fn glyphs_custom_layout<'a, 'b, S, L>(
        &'b mut self,
        section: S,
        custom_layout: &L,
    ) -> SectionGlyphIter<'b>
    where
        L: GlyphPositioner + std::hash::Hash,
        S: Into<Cow<'a, Section<'a>>>,
    {
        match self {
            GlyphBrush::Compatibility { glyph_brush, .. } => {
                glyph_brush.glyphs_custom_layout(section, custom_layout)
            }
            GlyphBrush::Core { glyph_brush, .. } => {
                glyph_brush.glyphs_custom_layout(section, custom_layout)
            }
        }
    }

    #[inline]
    fn glyph_bounds_custom_layout<'a, S, L>(
        &mut self,
        section: S,
        custom_layout: &L,
    ) -> Option<Rect>
    where
        L: GlyphPositioner + std::hash::Hash,
        S: Into<Cow<'a, Section<'a>>>,
    {
        match self {
            GlyphBrush::Compatibility { glyph_brush, .. } => {
                glyph_brush.glyph_bounds_custom_layout(section, custom_layout)
            }
            GlyphBrush::Core { glyph_brush, .. } => {
                glyph_brush.glyph_bounds_custom_layout(section, custom_layout)
            }
        }
    }

    #[inline]
    fn fonts(&self) -> &[F] {
        match self {
            GlyphBrush::Compatibility { glyph_brush, .. } => {
                glyph_brush.fonts()
            }
            GlyphBrush::Core { glyph_brush, .. } => glyph_brush.fonts(),
        }
    }
}

impl<F, H> std::fmt::Debug for GlyphBrush<F, H> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GlyphBrush")
    }
}
