// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! See [`UiRenderer`] docs.

use crate::{
    asset::untyped::ResourceKind,
    core::{
        algebra::{Matrix4, Vector2, Vector4},
        color::Color,
        math::Rect,
        sstorage::ImmutableString,
    },
    gui::{
        brush::Brush,
        draw::{CommandTexture, DrawingContext},
    },
    renderer::{
        cache::{
            shader::{binding, property, PropertyGroup, RenderMaterial, RenderPassContainer},
            uniform::UniformBufferCache,
        },
        flat_shader::FlatShader,
        framework::{
            buffer::BufferUsage,
            error::FrameworkError,
            framebuffer::{BufferLocation, GpuFrameBuffer, ResourceBindGroup, ResourceBinding},
            geometry_buffer::{
                AttributeDefinition, AttributeKind, ElementsDescriptor, GeometryBufferDescriptor,
                GpuGeometryBuffer, VertexBufferData, VertexBufferDescriptor,
            },
            server::GraphicsServer,
            uniform::StaticUniformBuffer,
            BlendFactor, BlendFunc, BlendParameters, ColorMask, CompareFunc, DrawParameters,
            ElementRange, ScissorBox, StencilAction, StencilFunc, StencilOp,
        },
        FallbackResources, RenderPassStatistics, TextureCache,
    },
    resource::texture::{Texture, TextureKind, TexturePixelKind, TextureResource},
};

/// User interface renderer allows you to render drawing context in specified render target.
pub struct UiRenderer {
    render_passes: RenderPassContainer,
    geometry_buffer: GpuGeometryBuffer,
    clipping_geometry_buffer: GpuGeometryBuffer,
}

/// A set of parameters to render a specified user interface drawing context.
pub struct UiRenderContext<'a, 'b, 'c> {
    /// Graphics server.
    pub server: &'a dyn GraphicsServer,
    /// Viewport to where render the user interface.
    pub viewport: Rect<i32>,
    /// Frame buffer to where render the user interface.
    pub frame_buffer: &'b GpuFrameBuffer,
    /// Width of the frame buffer to where render the user interface.
    pub frame_width: f32,
    /// Height of the frame buffer to where render the user interface.
    pub frame_height: f32,
    /// Drawing context of a user interface.
    pub drawing_context: &'c DrawingContext,
    /// Fallback textures.
    pub fallback_resources: &'a FallbackResources,
    /// GPU texture cache.
    pub texture_cache: &'a mut TextureCache,
    /// A reference to the cache of uniform buffers.
    pub uniform_buffer_cache: &'a mut UniformBufferCache,
    /// A reference to the shader that will be used to draw clipping geometry.
    pub flat_shader: &'a FlatShader,
}

impl UiRenderer {
    pub(in crate::renderer) fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        let geometry_buffer_desc = GeometryBufferDescriptor {
            elements: ElementsDescriptor::Triangles(&[]),
            buffers: &[VertexBufferDescriptor {
                usage: BufferUsage::DynamicDraw,
                attributes: &[
                    AttributeDefinition {
                        location: 0,
                        kind: AttributeKind::Float,
                        component_count: 2,
                        normalized: false,
                        divisor: 0,
                    },
                    AttributeDefinition {
                        location: 1,
                        kind: AttributeKind::Float,
                        component_count: 2,
                        normalized: false,
                        divisor: 0,
                    },
                    AttributeDefinition {
                        location: 2,
                        kind: AttributeKind::UnsignedByte,
                        component_count: 4,
                        normalized: true, // Make sure [0; 255] -> [0; 1]
                        divisor: 0,
                    },
                ],
                data: VertexBufferData::new::<crate::gui::draw::Vertex>(None),
            }],
            usage: BufferUsage::DynamicDraw,
        };

        let clipping_geometry_buffer_desc = GeometryBufferDescriptor {
            elements: ElementsDescriptor::Triangles(&[]),
            buffers: &[VertexBufferDescriptor {
                usage: BufferUsage::DynamicDraw,
                attributes: &[
                    // We're interested only in position. Fragment shader won't run for clipping geometry anyway.
                    AttributeDefinition {
                        location: 0,
                        kind: AttributeKind::Float,
                        component_count: 2,
                        normalized: false,
                        divisor: 0,
                    },
                ],
                data: VertexBufferData::new::<crate::gui::draw::Vertex>(None),
            }],
            usage: BufferUsage::DynamicDraw,
        };

        Ok(Self {
            geometry_buffer: server.create_geometry_buffer(geometry_buffer_desc)?,
            clipping_geometry_buffer: server
                .create_geometry_buffer(clipping_geometry_buffer_desc)?,
            render_passes: RenderPassContainer::from_str(
                server,
                include_str!("shaders/ui.shader"),
            )?,
        })
    }

    /// Renders given UI's drawing context to specified frame buffer.
    pub fn render(
        &mut self,
        args: UiRenderContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let UiRenderContext {
            server,
            viewport,
            frame_buffer,
            frame_width,
            frame_height,
            drawing_context,
            fallback_resources,
            texture_cache,
            uniform_buffer_cache,
            flat_shader,
        } = args;

        let mut statistics = RenderPassStatistics::default();

        self.geometry_buffer
            .set_buffer_data_of_type(0, drawing_context.get_vertices());
        self.geometry_buffer
            .set_triangles(drawing_context.get_triangles());

        let ortho = Matrix4::new_orthographic(0.0, frame_width, frame_height, 0.0, -1.0, 1.0);
        let resolution = Vector2::new(frame_width, frame_height);

        for cmd in drawing_context.get_commands() {
            let mut diffuse_texture = &fallback_resources.white_dummy;
            let mut is_font_texture = false;

            let mut clip_bounds = cmd.clip_bounds;
            clip_bounds.position.x = clip_bounds.position.x.floor();
            clip_bounds.position.y = clip_bounds.position.y.floor();
            clip_bounds.size.x = clip_bounds.size.x.ceil();
            clip_bounds.size.y = clip_bounds.size.y.ceil();

            let scissor_box = Some(ScissorBox {
                x: clip_bounds.position.x as i32,
                // Because OpenGL was designed for mathematicians, it has origin at lower left corner.
                y: viewport.size.y - (clip_bounds.position.y + clip_bounds.size.y) as i32,
                width: clip_bounds.size.x as i32,
                height: clip_bounds.size.y as i32,
            });

            let mut stencil_test = None;

            // Draw clipping geometry first if we have any. This is optional, because complex
            // clipping is very rare and in most cases scissor test will do the job.
            if let Some(clipping_geometry) = cmd.clipping_geometry.as_ref() {
                frame_buffer.clear(viewport, None, None, Some(0));

                self.clipping_geometry_buffer
                    .set_buffer_data_of_type(0, &clipping_geometry.vertex_buffer);
                self.clipping_geometry_buffer
                    .set_triangles(&clipping_geometry.triangle_buffer);

                let uniform_buffer =
                    uniform_buffer_cache.write(StaticUniformBuffer::<256>::new().with(&ortho))?;

                // Draw
                statistics += frame_buffer.draw(
                    &self.clipping_geometry_buffer,
                    viewport,
                    &flat_shader.program,
                    &DrawParameters {
                        cull_face: None,
                        color_write: ColorMask::all(false),
                        depth_write: false,
                        stencil_test: None,
                        depth_test: None,
                        blend: None,
                        stencil_op: StencilOp {
                            zpass: StencilAction::Incr,
                            ..Default::default()
                        },
                        scissor_box,
                    },
                    &[ResourceBindGroup {
                        bindings: &[ResourceBinding::Buffer {
                            buffer: uniform_buffer,
                            binding: BufferLocation::Auto {
                                shader_location: flat_shader.uniform_buffer_binding,
                            },
                            data_usage: Default::default(),
                        }],
                    }],
                    ElementRange::Full,
                )?;

                // Make sure main geometry will be drawn only on marked pixels.
                stencil_test = Some(StencilFunc {
                    func: CompareFunc::Equal,
                    ref_value: 1,
                    ..Default::default()
                });
            }

            match &cmd.texture {
                CommandTexture::Font {
                    font,
                    page_index,
                    height,
                } => {
                    if let Some(font) = font.state().data() {
                        let page_size = font.page_size() as u32;
                        if let Some(page) = font
                            .atlases
                            .get_mut(height)
                            .and_then(|atlas| atlas.pages.get_mut(*page_index))
                        {
                            if page.texture.is_none() || page.modified {
                                if let Some(details) = Texture::from_bytes(
                                    TextureKind::Rectangle {
                                        width: page_size,
                                        height: page_size,
                                    },
                                    TexturePixelKind::R8,
                                    page.pixels.clone(),
                                ) {
                                    page.texture = Some(
                                        TextureResource::new_ok(ResourceKind::Embedded, details)
                                            .into(),
                                    );
                                    page.modified = false;
                                }
                            }
                            if let Some(texture) = texture_cache.get(
                                server,
                                &page
                                    .texture
                                    .as_ref()
                                    .unwrap()
                                    .try_cast::<Texture>()
                                    .unwrap(),
                            ) {
                                diffuse_texture = texture;
                            }
                            is_font_texture = true;
                        }
                    }
                }
                CommandTexture::Texture(texture) => {
                    if let Some(texture) = texture_cache.get(server, texture) {
                        diffuse_texture = texture;
                    }
                }
                _ => (),
            }

            let mut raw_stops = [0.0; 16];
            let mut raw_colors = [Vector4::default(); 16];
            let bounds_max = cmd.bounds.right_bottom_corner();

            let (gradient_origin, gradient_end) = match cmd.brush {
                Brush::Solid(_) => (Vector2::default(), Vector2::default()),
                Brush::LinearGradient { from, to, .. } => (from, to),
                Brush::RadialGradient { center, .. } => (center, Vector2::default()),
            };

            let params = DrawParameters {
                cull_face: None,
                color_write: ColorMask::all(true),
                depth_write: false,
                stencil_test,
                depth_test: None,
                blend: Some(BlendParameters {
                    func: BlendFunc::new(BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
                    ..Default::default()
                }),
                stencil_op: Default::default(),
                scissor_box,
            };

            let solid_color = match cmd.brush {
                Brush::Solid(color) => color,
                _ => Color::WHITE,
            };
            let gradient_colors = match cmd.brush {
                Brush::Solid(_) => &raw_colors,
                Brush::LinearGradient { ref stops, .. }
                | Brush::RadialGradient { ref stops, .. } => {
                    for (i, point) in stops.iter().enumerate() {
                        raw_colors[i] = point.color.as_frgba();
                    }
                    &raw_colors
                }
            };
            let gradient_stops = match cmd.brush {
                Brush::Solid(_) => &raw_stops,
                Brush::LinearGradient { ref stops, .. }
                | Brush::RadialGradient { ref stops, .. } => {
                    for (i, point) in stops.iter().enumerate() {
                        raw_stops[i] = point.stop;
                    }
                    &raw_stops
                }
            };
            let brush_type = match cmd.brush {
                Brush::Solid(_) => 0,
                Brush::LinearGradient { .. } => 1,
                Brush::RadialGradient { .. } => 2,
            };
            let gradient_point_count = match cmd.brush {
                Brush::Solid(_) => 0,
                Brush::LinearGradient { ref stops, .. }
                | Brush::RadialGradient { ref stops, .. } => stops.len() as i32,
            };

            let properties = PropertyGroup::from([
                property("worldViewProjection", &ortho),
                property("solidColor", &solid_color),
                property("gradientColors", gradient_colors.as_slice()),
                property("gradientStops", gradient_stops.as_slice()),
                property("gradientOrigin", &gradient_origin),
                property("gradientEnd", &gradient_end),
                property("resolution", &resolution),
                property("boundsMin", &cmd.bounds.position),
                property("boundsMax", &bounds_max),
                property("isFont", &is_font_texture),
                property("opacity", &cmd.opacity),
                property("brushType", &brush_type),
                property("gradientPointCount", &gradient_point_count),
            ]);

            let material = RenderMaterial::from([
                binding("diffuseTexture", diffuse_texture),
                binding("properties", &properties),
            ]);

            self.render_passes.run_pass(
                &ImmutableString::new("Primary"),
                frame_buffer,
                &self.geometry_buffer,
                viewport,
                &material,
                uniform_buffer_cache,
                ElementRange::Specific {
                    offset: cmd.triangles.start,
                    count: cmd.triangles.end - cmd.triangles.start,
                },
                Some(&params),
            )?;
        }

        Ok(statistics)
    }
}
