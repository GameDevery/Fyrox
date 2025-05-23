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

//! Rectangle is the simplest "2D" node, it can be used to create "2D" graphics. 2D is in quotes
//! here because the node is actually a 3D node, like everything else in the engine.
//!
//! See [`Rectangle`] docs for more info.

use crate::scene::node::constructor::NodeConstructor;
use crate::{
    core::{
        algebra::{Point3, Vector2, Vector3},
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, Rect, TriangleDefinition},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    material::{self, Material, MaterialResource},
    renderer::{self, bundle::RenderContext},
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        mesh::buffer::{
            VertexAttributeDataType, VertexAttributeDescriptor, VertexAttributeUsage, VertexTrait,
        },
        mesh::RenderPath,
        node::{Node, NodeTrait, RdcControlFlow},
    },
};
use bytemuck::{Pod, Zeroable};
use fyrox_core::value_as_u8_slice;

use fyrox_graph::constructor::ConstructorProvider;
use fyrox_graph::BaseSceneGraph;
use std::{
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
};

/// A vertex for static meshes.
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
#[repr(C)] // OpenGL expects this structure packed as in C
pub struct RectangleVertex {
    /// Position of vertex in local coordinates.
    pub position: Vector3<f32>,
    /// Texture coordinates.
    pub tex_coord: Vector2<f32>,
    /// Diffuse color.
    pub color: Color,
}

impl VertexTrait for RectangleVertex {
    fn layout() -> &'static [VertexAttributeDescriptor] {
        &[
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Position,
                data_type: VertexAttributeDataType::F32,
                size: 3,
                divisor: 0,
                shader_location: 0,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::TexCoord0,
                data_type: VertexAttributeDataType::F32,
                size: 2,
                divisor: 0,
                shader_location: 1,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Color,
                data_type: VertexAttributeDataType::U8,
                size: 4,
                divisor: 0,
                shader_location: 2,
                normalized: true,
            },
        ]
    }
}

impl PartialEq for RectangleVertex {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
            && self.tex_coord == other.tex_coord
            && self.color == other.color
    }
}

// This is safe because Vertex is tightly packed struct with C representation
// there is no padding bytes which may contain garbage data. This is strictly
// required because vertices will be directly passed on GPU.
impl Hash for RectangleVertex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        #[allow(unsafe_code)]
        unsafe {
            let bytes = self as *const Self as *const u8;
            state.write(std::slice::from_raw_parts(
                bytes,
                std::mem::size_of::<Self>(),
            ))
        }
    }
}

/// Rectangle is the simplest "2D" node, it can be used to create "2D" graphics. 2D is in quotes
/// here because the node is actually a 3D node, like everything else in the engine.
///
/// ## Material
///
/// Rectangles could use an arbitrary material for rendering, which means that you have full control
/// on how the rectangle will be rendered on screen.
///
/// By default, the rectangle uses standard 2D material which has only one property - `diffuseTexture`.
/// You could use it to set a texture for your rectangle:
///
/// ```rust
/// # use fyrox_impl::{
/// #     core::sstorage::ImmutableString,
/// #     material::{shader::SamplerFallback, MaterialProperty},
/// #     resource::texture::TextureResource,
/// #     scene::dim2::rectangle::Rectangle,
/// # };
/// #
/// fn set_texture(rect: &mut Rectangle, texture: Option<TextureResource>) {
///     rect.material()
///         .data_ref()
///         .bind("diffuseTexture", texture);
/// }
/// ```
///
/// The same property could also be changed in the editor using the Material Editor invoked from
/// the `Material` property in the Inspector.
///
/// ## Performance
///
/// Rectangles use batching to let you draw tons of rectangles with high performance.
///
/// ## Specifying region for rendering
///
/// You can specify a portion of the texture that will be used for rendering using [`Self::set_uv_rect`]
/// method. This is especially useful if you need to create sprite sheet animation, you use the single
/// image, but just changing portion for rendering. Keep in mind that the coordinates are normalized
/// which means `[0; 0]` corresponds to top-left corner of the texture and `[1; 1]` corresponds to
/// right-bottom corner.
#[derive(Reflect, Debug, Clone, ComponentProvider)]
#[reflect(derived_type = "Node")]
pub struct Rectangle {
    base: Base,

    #[reflect(setter = "set_color")]
    color: InheritableVariable<Color>,

    #[reflect(setter = "set_uv_rect")]
    uv_rect: InheritableVariable<Rect<f32>>,

    material: InheritableVariable<MaterialResource>,
}

impl Visit for Rectangle {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        if region.is_reading() {
            if let Some(material) =
                material::visit_old_texture_as_material(&mut region, Material::standard_2d)
            {
                self.material = material.into();
            } else {
                self.material.visit("Material", &mut region)?;
            }
        } else {
            self.material.visit("Material", &mut region)?;
        }

        self.base.visit("Base", &mut region)?;
        self.color.visit("Color", &mut region)?;
        let _ = self.uv_rect.visit("UvRect", &mut region);

        Ok(())
    }
}

impl Default for Rectangle {
    fn default() -> Self {
        Self {
            base: Default::default(),
            color: Default::default(),
            uv_rect: InheritableVariable::new_modified(Rect::new(0.0, 0.0, 1.0, 1.0)),
            material: InheritableVariable::new_modified(MaterialResource::new_ok(
                Uuid::new_v4(),
                Default::default(),
                Material::standard_2d(),
            )),
        }
    }
}

impl Deref for Rectangle {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Rectangle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl TypeUuidProvider for Rectangle {
    fn type_uuid() -> Uuid {
        uuid!("bb57b5e0-367a-4490-bf30-7f547407d5b5")
    }
}

impl Rectangle {
    /// Returns current color of the rectangle.
    pub fn color(&self) -> Color {
        *self.color
    }

    /// Returns a reference to the current material used by the rectangle.
    pub fn material(&self) -> &InheritableVariable<MaterialResource> {
        &self.material
    }

    /// Returns a reference to the current material used by the rectangle.
    pub fn material_mut(&mut self) -> &mut InheritableVariable<MaterialResource> {
        &mut self.material
    }

    /// Sets color of the rectangle.
    pub fn set_color(&mut self, color: Color) -> Color {
        self.color.set_value_and_mark_modified(color)
    }

    /// Returns a rectangle that defines the region in texture which will be rendered. The coordinates are normalized
    /// which means `[0; 0]` corresponds to top-left corner of the texture and `[1; 1]` corresponds to right-bottom
    /// corner.
    pub fn uv_rect(&self) -> Rect<f32> {
        *self.uv_rect
    }

    /// Sets a rectangle that defines the region in texture which will be rendered. The coordinates are normalized
    /// which means `[0; 0]` corresponds to top-left corner of the texture and `[1; 1]` corresponds to right-bottom
    /// corner.
    ///
    /// The coordinates can exceed `[1; 1]` boundary to create tiling effect (keep in mind that tiling should be
    /// enabled in texture options).
    ///
    /// The default value is `(0, 0, 1, 1)` rectangle which corresponds to entire texture.
    pub fn set_uv_rect(&mut self, uv_rect: Rect<f32>) -> Rect<f32> {
        self.uv_rect.set_value_and_mark_modified(uv_rect)
    }
}

impl ConstructorProvider<Node, Graph> for Rectangle {
    fn constructor() -> NodeConstructor {
        NodeConstructor::new::<Self>()
            .with_variant("Rectangle (2D Sprite)", |_| {
                RectangleBuilder::new(BaseBuilder::new().with_name("Sprite (2D)"))
                    .build_node()
                    .into()
            })
            .with_group("2D")
    }
}

impl NodeTrait for Rectangle {
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox::unit()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.local_bounding_box()
            .transform(&self.global_transform())
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn collect_render_data(&self, ctx: &mut RenderContext) -> RdcControlFlow {
        if !self.should_be_rendered(ctx.frustum, ctx.render_mask) {
            return RdcControlFlow::Continue;
        }

        if renderer::is_shadow_pass(ctx.render_pass_name) {
            return RdcControlFlow::Continue;
        }

        let global_transform = self.global_transform();

        type Vertex = RectangleVertex;

        let vertices = [
            Vertex {
                position: global_transform
                    .transform_point(&Point3::new(-0.5, 0.5, 0.0))
                    .coords,
                tex_coord: self.uv_rect.right_top_corner(),
                color: *self.color,
            },
            Vertex {
                position: global_transform
                    .transform_point(&Point3::new(0.5, 0.5, 0.0))
                    .coords,
                tex_coord: self.uv_rect.left_top_corner(),
                color: *self.color,
            },
            Vertex {
                position: global_transform
                    .transform_point(&Point3::new(0.5, -0.5, 0.0))
                    .coords,
                tex_coord: self.uv_rect.left_bottom_corner(),
                color: *self.color,
            },
            Vertex {
                position: global_transform
                    .transform_point(&Point3::new(-0.5, -0.5, 0.0))
                    .coords,
                tex_coord: self.uv_rect.right_bottom_corner(),
                color: *self.color,
            },
        ];

        let triangles = [TriangleDefinition([0, 1, 2]), TriangleDefinition([2, 3, 0])];

        let sort_index = ctx.calculate_sorting_index(self.global_position());

        ctx.storage.push_triangles(
            ctx.dynamic_surface_cache,
            Vertex::layout(),
            &self.material,
            RenderPath::Forward,
            sort_index,
            self.handle(),
            &mut move |mut vertex_buffer, mut triangle_buffer| {
                let start_vertex_index = vertex_buffer.vertex_count();

                for vertex in vertices.iter() {
                    vertex_buffer
                        .push_vertex_raw(value_as_u8_slice(vertex))
                        .unwrap();
                }

                triangle_buffer
                    .push_triangles_iter_with_offset(start_vertex_index, triangles.into_iter());
            },
        );

        RdcControlFlow::Continue
    }
}

/// Allows you to create rectangle in declarative manner.
pub struct RectangleBuilder {
    base_builder: BaseBuilder,
    color: Color,
    uv_rect: Rect<f32>,
    material: MaterialResource,
}

impl RectangleBuilder {
    /// Creates new rectangle builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            color: Color::WHITE,
            uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            material: MaterialResource::new_ok(
                Uuid::new_v4(),
                Default::default(),
                Material::standard_2d(),
            ),
        }
    }

    /// Sets desired color of the rectangle.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Sets desired portion of the texture for the rectangle. See [`Rectangle::set_uv_rect`]
    /// for more info.
    pub fn with_uv_rect(mut self, uv_rect: Rect<f32>) -> Self {
        self.uv_rect = uv_rect;
        self
    }

    /// Sets the desired material of the rectangle.
    pub fn with_material(mut self, material: MaterialResource) -> Self {
        self.material = material;
        self
    }

    /// Creates new [`Rectangle`] instance.
    pub fn build_rectangle(self) -> Rectangle {
        Rectangle {
            base: self.base_builder.build_base(),
            color: self.color.into(),
            uv_rect: self.uv_rect.into(),
            material: self.material.into(),
        }
    }

    /// Creates new [`Rectangle`] instance.
    pub fn build_node(self) -> Node {
        Node::new(self.build_rectangle())
    }

    /// Creates new [`Rectangle`] instance and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
