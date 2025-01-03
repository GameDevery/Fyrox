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

use crate::{
    buffer::Buffer,
    core::{color::Color, math::Rect},
    error::FrameworkError,
    geometry_buffer::{DrawCallStatistics, GeometryBuffer},
    gpu_program::{GpuProgram, UniformLocation},
    gpu_texture::{CubeMapFace, GpuTexture},
    DrawParameters, ElementRange,
};
use std::any::Any;
use std::{cell::RefCell, rc::Rc};

#[derive(Copy, Clone, PartialOrd, PartialEq, Hash, Debug, Eq)]
pub enum AttachmentKind {
    Color,
    DepthStencil,
    Depth,
}

pub struct Attachment {
    pub kind: AttachmentKind,
    pub texture: Rc<RefCell<dyn GpuTexture>>,
}

#[derive(Default)]
pub enum BufferDataUsage {
    #[default]
    UseEverything,
    UseSegment {
        offset: usize,
        size: usize,
    },
}

pub enum TextureShaderLocation {
    Uniform(UniformLocation),
    ExplicitBinding(usize),
}

// TODO: Remove when raw shaders will be replaced with Fyrox-native ones.
pub enum BufferLocation {
    Auto { shader_location: usize },
    Explicit { binding: usize },
}

pub enum ResourceBinding<'a> {
    Texture {
        texture: Rc<RefCell<dyn GpuTexture>>,
        shader_location: TextureShaderLocation,
    },
    Buffer {
        buffer: &'a dyn Buffer,
        binding: BufferLocation,
        data_usage: BufferDataUsage,
    },
}

impl ResourceBinding<'_> {
    pub fn texture(
        texture: &Rc<RefCell<dyn GpuTexture>>,
        shader_location: &UniformLocation,
    ) -> Self {
        Self::Texture {
            texture: texture.clone(),
            shader_location: TextureShaderLocation::Uniform(shader_location.clone()),
        }
    }

    pub fn texture_with_binding(texture: &Rc<RefCell<dyn GpuTexture>>, binding: usize) -> Self {
        Self::Texture {
            texture: texture.clone(),
            shader_location: TextureShaderLocation::ExplicitBinding(binding),
        }
    }
}

pub struct ResourceBindGroup<'a> {
    pub bindings: &'a [ResourceBinding<'a>],
}

pub trait FrameBuffer: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn color_attachments(&self) -> &[Attachment];
    fn depth_attachment(&self) -> Option<&Attachment>;
    fn set_cubemap_face(&mut self, attachment_index: usize, face: CubeMapFace);
    fn blit_to(
        &self,
        dest: &dyn FrameBuffer,
        src_x0: i32,
        src_y0: i32,
        src_x1: i32,
        src_y1: i32,
        dst_x0: i32,
        dst_y0: i32,
        dst_x1: i32,
        dst_y1: i32,
        copy_color: bool,
        copy_depth: bool,
        copy_stencil: bool,
    );
    fn clear(
        &mut self,
        viewport: Rect<i32>,
        color: Option<Color>,
        depth: Option<f32>,
        stencil: Option<i32>,
    );
    fn draw(
        &mut self,
        geometry: &dyn GeometryBuffer,
        viewport: Rect<i32>,
        program: &dyn GpuProgram,
        params: &DrawParameters,
        resources: &[ResourceBindGroup],
        element_range: ElementRange,
    ) -> Result<DrawCallStatistics, FrameworkError>;
    fn draw_instances(
        &mut self,
        count: usize,
        geometry: &dyn GeometryBuffer,
        viewport: Rect<i32>,
        program: &dyn GpuProgram,
        params: &DrawParameters,
        resources: &[ResourceBindGroup],
    ) -> DrawCallStatistics;
}
