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

use crate::renderer::framework::GeometryBufferExt;
use crate::{
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        math::Rect,
        sstorage::ImmutableString,
    },
    renderer::{
        framework::{
            error::FrameworkError,
            framebuffer::FrameBuffer,
            geometry_buffer::GeometryBuffer,
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::GpuTexture,
            state::GlGraphicsServer,
            DrawParameters, ElementRange,
        },
        RenderPassStatistics,
    },
    scene::mesh::surface::SurfaceData,
};
use fyrox_graphics::buffer::BufferUsage;
use std::{cell::RefCell, rc::Rc};

struct FxaaShader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub screen_texture: UniformLocation,
    pub inverse_screen_size: UniformLocation,
}

impl FxaaShader {
    pub fn new(server: &GlGraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/fxaa_fs.glsl");
        let vertex_source = include_str!("shaders/flat_vs.glsl");

        let program =
            GpuProgram::from_source(server, "FXAAShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program
                .uniform_location(server, &ImmutableString::new("worldViewProjection"))?,
            screen_texture: program
                .uniform_location(server, &ImmutableString::new("screenTexture"))?,
            inverse_screen_size: program
                .uniform_location(server, &ImmutableString::new("inverseScreenSize"))?,
            program,
        })
    }
}

pub struct FxaaRenderer {
    shader: FxaaShader,
    quad: GeometryBuffer,
}

impl FxaaRenderer {
    pub fn new(server: &GlGraphicsServer) -> Result<Self, FrameworkError> {
        Ok(Self {
            shader: FxaaShader::new(server)?,
            quad: GeometryBuffer::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                BufferUsage::StaticDraw,
                server,
            )?,
        })
    }

    pub(crate) fn render(
        &self,
        server: &GlGraphicsServer,
        viewport: Rect<i32>,
        frame_texture: Rc<RefCell<dyn GpuTexture>>,
        frame_buffer: &mut FrameBuffer,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut statistics = RenderPassStatistics::default();

        let frame_matrix = Matrix4::new_orthographic(
            0.0,
            viewport.w() as f32,
            viewport.h() as f32,
            0.0,
            -1.0,
            1.0,
        ) * Matrix4::new_nonuniform_scaling(&Vector3::new(
            viewport.w() as f32,
            viewport.h() as f32,
            0.0,
        ));

        statistics += frame_buffer.draw(
            &self.quad,
            server,
            viewport,
            &self.shader.program,
            &DrawParameters {
                cull_face: None,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: None,
                depth_test: None,
                blend: None,
                stencil_op: Default::default(),
                scissor_box: None,
            },
            ElementRange::Full,
            |mut program_binding| {
                program_binding
                    .set_matrix4(&self.shader.wvp_matrix, &frame_matrix)
                    .set_vector2(
                        &self.shader.inverse_screen_size,
                        &Vector2::new(1.0 / viewport.w() as f32, 1.0 / viewport.h() as f32),
                    )
                    .set_texture(&self.shader.screen_texture, &frame_texture);
            },
        )?;

        Ok(statistics)
    }
}
