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

//! GBuffer Layout:
//!
//! RT0: sRGBA8 - Diffuse color (xyz)
//! RT1: RGBA8 - Normal (xyz)
//! RT2: RGBA16F - Ambient light + emission (both in xyz)
//! RT3: RGBA8 - Metallic (x) + Roughness (y) + Ambient Occlusion (z)
//! RT4: R8UI - Decal mask (x)
//!
//! Every alpha channel is used for layer blending for terrains. This is inefficient, but for
//! now I don't know better solution.

use crate::renderer::framework::GeometryBufferExt;
use crate::{
    core::{
        algebra::{Matrix4, Vector2},
        color::Color,
        math::{Matrix4Ext, Rect},
        sstorage::ImmutableString,
    },
    renderer::{
        bundle::{BundleRenderContext, RenderDataBundleStorage, SurfaceInstanceData},
        cache::shader::ShaderCache,
        debug_renderer::DebugRenderer,
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, FrameBuffer},
            geometry_buffer::GeometryBuffer,
            gpu_texture::{
                Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
                PixelKind, WrapMode,
            },
            state::GlGraphicsServer,
            BlendFactor, BlendFunc, BlendParameters, DrawParameters, ElementRange,
        },
        gbuffer::decal::DecalShader,
        occlusion::OcclusionTester,
        storage::MatrixStorageCache,
        GeometryCache, QualitySettings, RenderPassStatistics, TextureCache,
    },
    scene::{
        camera::Camera,
        decal::Decal,
        graph::Graph,
        mesh::{surface::SurfaceData, RenderPath},
    },
};
use fxhash::FxHashSet;
use fyrox_graphics::buffer::BufferUsage;
use fyrox_graphics::state::GraphicsServer;
use std::{cell::RefCell, rc::Rc};

mod decal;

pub struct GBuffer {
    framebuffer: FrameBuffer,
    decal_framebuffer: FrameBuffer,
    pub width: i32,
    pub height: i32,
    cube: GeometryBuffer,
    decal_shader: DecalShader,
    render_pass_name: ImmutableString,
    occlusion_tester: OcclusionTester,
}

pub(crate) struct GBufferRenderContext<'a, 'b> {
    pub state: &'a GlGraphicsServer,
    pub camera: &'b Camera,
    pub geom_cache: &'a mut GeometryCache,
    pub bundle_storage: &'a RenderDataBundleStorage,
    pub texture_cache: &'a mut TextureCache,
    pub shader_cache: &'a mut ShaderCache,
    pub white_dummy: Rc<RefCell<dyn GpuTexture>>,
    pub normal_dummy: Rc<RefCell<dyn GpuTexture>>,
    pub black_dummy: Rc<RefCell<dyn GpuTexture>>,
    pub volume_dummy: Rc<RefCell<dyn GpuTexture>>,
    pub quality_settings: &'a QualitySettings,
    pub graph: &'b Graph,
    pub matrix_storage: &'a mut MatrixStorageCache,
    #[allow(dead_code)]
    pub screen_space_debug_renderer: &'a mut DebugRenderer,
    pub unit_quad: &'a GeometryBuffer,
}

impl GBuffer {
    pub fn new(
        server: &GlGraphicsServer,
        width: usize,
        height: usize,
    ) -> Result<Self, FrameworkError> {
        let depth_stencil = server.create_texture(
            GpuTextureKind::Rectangle { width, height },
            PixelKind::D24S8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        depth_stencil
            .borrow_mut()
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge);
        depth_stencil
            .borrow_mut()
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let diffuse_texture = server.create_texture(
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        diffuse_texture
            .borrow_mut()
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge);
        diffuse_texture
            .borrow_mut()
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let normal_texture = server.create_texture(
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        normal_texture
            .borrow_mut()
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge);
        normal_texture
            .borrow_mut()
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let ambient_texture = server.create_texture(
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA16F,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        ambient_texture
            .borrow_mut()
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge);
        ambient_texture
            .borrow_mut()
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let decal_mask_texture = server.create_texture(
            GpuTextureKind::Rectangle { width, height },
            PixelKind::R8UI,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        decal_mask_texture
            .borrow_mut()
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge);
        decal_mask_texture
            .borrow_mut()
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let material_texture = server.create_texture(
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        material_texture
            .borrow_mut()
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge);
        material_texture
            .borrow_mut()
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let framebuffer = FrameBuffer::new(
            server,
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: depth_stencil,
            }),
            vec![
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: diffuse_texture.clone(),
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: normal_texture.clone(),
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: ambient_texture,
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: material_texture,
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: decal_mask_texture,
                },
            ],
        )?;

        let decal_framebuffer = FrameBuffer::new(
            server,
            None,
            vec![
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: diffuse_texture,
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: normal_texture,
                },
            ],
        )?;

        Ok(Self {
            framebuffer,
            width: width as i32,
            height: height as i32,
            decal_shader: DecalShader::new(server)?,
            cube: GeometryBuffer::from_surface_data(
                &SurfaceData::make_cube(Matrix4::identity()),
                BufferUsage::StaticDraw,
                server,
            )?,
            decal_framebuffer,
            render_pass_name: ImmutableString::new("GBuffer"),
            occlusion_tester: OcclusionTester::new(server, width, height, 16)?,
        })
    }

    pub fn framebuffer(&self) -> &FrameBuffer {
        &self.framebuffer
    }

    pub fn depth(&self) -> Rc<RefCell<dyn GpuTexture>> {
        self.framebuffer.depth_attachment().unwrap().texture.clone()
    }

    pub fn diffuse_texture(&self) -> Rc<RefCell<dyn GpuTexture>> {
        self.framebuffer.color_attachments()[0].texture.clone()
    }

    pub fn normal_texture(&self) -> Rc<RefCell<dyn GpuTexture>> {
        self.framebuffer.color_attachments()[1].texture.clone()
    }

    pub fn ambient_texture(&self) -> Rc<RefCell<dyn GpuTexture>> {
        self.framebuffer.color_attachments()[2].texture.clone()
    }

    pub fn material_texture(&self) -> Rc<RefCell<dyn GpuTexture>> {
        self.framebuffer.color_attachments()[3].texture.clone()
    }

    pub fn decal_mask_texture(&self) -> Rc<RefCell<dyn GpuTexture>> {
        self.framebuffer.color_attachments()[4].texture.clone()
    }

    pub(crate) fn fill(
        &mut self,
        args: GBufferRenderContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut statistics = RenderPassStatistics::default();

        let GBufferRenderContext {
            state,
            camera,
            geom_cache,
            bundle_storage,
            texture_cache,
            shader_cache,
            quality_settings,
            white_dummy,
            normal_dummy,
            black_dummy,
            volume_dummy,
            graph,
            matrix_storage,
            unit_quad,
            ..
        } = args;

        let view_projection = camera.view_projection_matrix();

        if quality_settings.use_occlusion_culling {
            self.occlusion_tester
                .try_query_visibility_results(state, graph);
        };

        let viewport = Rect::new(0, 0, self.width, self.height);
        self.framebuffer.clear(
            state,
            viewport,
            Some(Color::from_rgba(0, 0, 0, 0)),
            Some(1.0),
            Some(0),
        );

        let inv_view = camera.inv_view_matrix().unwrap();

        let camera_up = inv_view.up();
        let camera_side = inv_view.side();

        let grid_cell = self
            .occlusion_tester
            .grid_cache
            .cell(camera.global_position());

        let instance_filter = |instance: &SurfaceInstanceData| {
            !quality_settings.use_occlusion_culling
                || grid_cell.map_or(true, |cell| cell.is_visible(instance.node_handle))
        };

        for bundle in bundle_storage
            .bundles
            .iter()
            .filter(|b| b.render_path == RenderPath::Deferred)
        {
            statistics += bundle.render_to_frame_buffer(
                state,
                geom_cache,
                shader_cache,
                instance_filter,
                BundleRenderContext {
                    texture_cache,
                    render_pass_name: &self.render_pass_name,
                    frame_buffer: &mut self.framebuffer,
                    viewport,
                    matrix_storage,
                    view_projection_matrix: &view_projection,
                    camera_position: &camera.global_position(),
                    camera_up_vector: &camera_up,
                    camera_side_vector: &camera_side,
                    z_near: camera.projection().z_near(),
                    use_pom: quality_settings.use_parallax_mapping,
                    light_position: &Default::default(),
                    normal_dummy: &normal_dummy,
                    white_dummy: &white_dummy,
                    black_dummy: &black_dummy,
                    volume_dummy: &volume_dummy,
                    light_data: None,
                    ambient_light: Color::WHITE, // TODO
                    scene_depth: None,           // TODO. Add z-pre-pass.
                    z_far: camera.projection().z_far(),
                },
            )?;
        }

        if quality_settings.use_occlusion_culling {
            let mut objects = FxHashSet::default();
            for bundle in bundle_storage.bundles.iter() {
                for instance in bundle.instances.iter() {
                    objects.insert(instance.node_handle);
                }
            }

            self.occlusion_tester.try_run_visibility_test(
                state,
                graph,
                None,
                unit_quad,
                objects.iter(),
                &self.framebuffer,
                camera.global_position(),
                view_projection,
            )?;
        }

        let inv_view_proj = view_projection.try_inverse().unwrap_or_default();
        let depth = self.depth();
        let decal_mask = self.decal_mask_texture();
        let resolution = Vector2::new(self.width as f32, self.height as f32);

        // Render decals after because we need to modify diffuse texture of G-Buffer and use depth texture
        // for rendering. We'll render in the G-Buffer, but depth will be used from final frame, since
        // decals do not modify depth (only diffuse and normal maps).
        let unit_cube = &self.cube;
        for decal in graph.linear_iter().filter_map(|n| n.cast::<Decal>()) {
            let shader = &self.decal_shader;
            let program = &self.decal_shader.program;

            let world_view_proj = view_projection * decal.global_transform();

            statistics += self.decal_framebuffer.draw(
                unit_cube,
                state,
                viewport,
                program,
                &DrawParameters {
                    cull_face: None,
                    color_write: Default::default(),
                    depth_write: false,
                    stencil_test: None,
                    depth_test: None,
                    blend: Some(BlendParameters {
                        func: BlendFunc::new(BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
                        ..Default::default()
                    }),
                    stencil_op: Default::default(),
                    scissor_box: None,
                },
                ElementRange::Full,
                |mut program_binding| {
                    program_binding
                        .set_matrix4(&shader.world_view_projection, &world_view_proj)
                        .set_matrix4(&shader.inv_view_proj, &inv_view_proj)
                        .set_matrix4(
                            &shader.inv_world_decal,
                            &decal.global_transform().try_inverse().unwrap_or_default(),
                        )
                        .set_vector2(&shader.resolution, &resolution)
                        .set_texture(&shader.scene_depth, &depth)
                        .set_texture(
                            &shader.diffuse_texture,
                            decal
                                .diffuse_texture()
                                .and_then(|t| texture_cache.get(state, t))
                                .unwrap_or(&white_dummy),
                        )
                        .set_texture(
                            &shader.normal_texture,
                            decal
                                .normal_texture()
                                .and_then(|t| texture_cache.get(state, t))
                                .unwrap_or(&normal_dummy),
                        )
                        .set_texture(&shader.decal_mask, &decal_mask)
                        .set_u32(&shader.layer_index, decal.layer() as u32)
                        .set_linear_color(&shader.color, &decal.color());
                },
            )?;
        }

        Ok(statistics)
    }
}
