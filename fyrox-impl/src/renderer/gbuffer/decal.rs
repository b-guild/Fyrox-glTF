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

use crate::core::sstorage::ImmutableString;
use crate::renderer::framework::{
    error::FrameworkError,
    gpu_program::{GpuProgram, UniformLocation},
    state::GlGraphicsServer,
};

pub struct DecalShader {
    pub world_view_projection: UniformLocation,
    pub scene_depth: UniformLocation,
    pub diffuse_texture: UniformLocation,
    pub normal_texture: UniformLocation,
    pub inv_view_proj: UniformLocation,
    pub inv_world_decal: UniformLocation,
    pub resolution: UniformLocation,
    pub color: UniformLocation,
    pub layer_index: UniformLocation,
    pub decal_mask: UniformLocation,
    pub program: GpuProgram,
}

impl DecalShader {
    pub fn new(server: &GlGraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/decal_fs.glsl");
        let vertex_source = include_str!("../shaders/decal_vs.glsl");

        let program =
            GpuProgram::from_source(server, "DecalShader", vertex_source, fragment_source)?;
        Ok(Self {
            world_view_projection: program
                .uniform_location(server, &ImmutableString::new("worldViewProjection"))?,
            scene_depth: program.uniform_location(server, &ImmutableString::new("sceneDepth"))?,
            diffuse_texture: program
                .uniform_location(server, &ImmutableString::new("diffuseTexture"))?,
            normal_texture: program
                .uniform_location(server, &ImmutableString::new("normalTexture"))?,
            inv_view_proj: program
                .uniform_location(server, &ImmutableString::new("invViewProj"))?,
            inv_world_decal: program
                .uniform_location(server, &ImmutableString::new("invWorldDecal"))?,
            resolution: program.uniform_location(server, &ImmutableString::new("resolution"))?,
            color: program.uniform_location(server, &ImmutableString::new("color"))?,
            layer_index: program.uniform_location(server, &ImmutableString::new("layerIndex"))?,
            decal_mask: program.uniform_location(server, &ImmutableString::new("decalMask"))?,
            program,
        })
    }
}
