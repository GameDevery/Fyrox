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

use crate::command::CommandContext;
use crate::fyrox::{
    core::{pool::Handle, sstorage::ImmutableString},
    material::{shader::SamplerFallback, PropertyValue},
    resource::texture::TextureResource,
    scene::{mesh::Mesh, node::Node},
};
use crate::{command::CommandTrait, scene::commands::GameSceneContext};

#[derive(Debug)]
enum TextureSet {
    Single(TextureResource),
    Multiple(Vec<Option<TextureResource>>),
}

#[derive(Debug)]
pub struct SetMeshTextureCommand {
    node: Handle<Node>,
    set: TextureSet,
}

impl SetMeshTextureCommand {
    pub fn new(node: Handle<Node>, texture: TextureResource) -> Self {
        Self {
            node,
            set: TextureSet::Single(texture),
        }
    }
}

impl CommandTrait for SetMeshTextureCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Texture".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        if let TextureSet::Single(texture) = &self.set {
            let mesh: &mut Mesh = context.scene.graph[self.node].as_mesh_mut();
            let old_set = mesh
                .surfaces_mut()
                .iter()
                .map(|s| {
                    s.material()
                        .data_ref()
                        .property_ref(&ImmutableString::new("diffuseTexture"))
                        .and_then(|p| {
                            if let PropertyValue::Sampler { value, .. } = p {
                                value.clone()
                            } else {
                                None
                            }
                        })
                })
                .collect();
            for surface in mesh.surfaces_mut() {
                surface
                    .material()
                    .data_ref()
                    .set_property("diffuseTexture", texture.clone())
                    .unwrap();
            }
            self.set = TextureSet::Multiple(old_set);
        } else {
            unreachable!()
        }
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        if let TextureSet::Multiple(set) = &self.set {
            let mesh: &mut Mesh = context.scene.graph[self.node].as_mesh_mut();
            let new_value = mesh.surfaces_mut()[0]
                .material()
                .data_ref()
                .property_ref(&ImmutableString::new("diffuseTexture"))
                .and_then(|p| {
                    if let PropertyValue::Sampler { value, .. } = p {
                        value.clone()
                    } else {
                        None
                    }
                })
                .unwrap();
            assert_eq!(mesh.surfaces_mut().len(), set.len());
            for (surface, old_texture) in mesh.surfaces_mut().iter_mut().zip(set) {
                surface
                    .material()
                    .data_ref()
                    .set_property(
                        "diffuseTexture",
                        PropertyValue::Sampler {
                            value: old_texture.clone(),
                            fallback: SamplerFallback::White,
                        },
                    )
                    .unwrap();
            }
            self.set = TextureSet::Single(new_value);
        } else {
            unreachable!()
        }
    }
}
