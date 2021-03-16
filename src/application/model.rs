use erupt::vk;

use std::mem::size_of;

use ultraviolet::mat::Mat4;

pub type Index = u32;

pub const VERTICES: [Vertex; 4] = [
    Vertex {
        _pos: [-0.5, -0.5, 0.5],
        _color: [1.0, 0.0, 0.0],
    },
    Vertex {
        _pos: [0.5, -0.5, 0.5],
        _color: [0.0, 1.0, 0.0],
    },
    Vertex {
        _pos: [0.5, 0.5, 0.5],
        _color: [0.0, 0.0, 1.0],
    },
    Vertex {
        _pos: [-0.5, 0.5, 0.5],
        _color: [1.0, 1.0, 1.0],
    },
];

pub const INDICIES: [Index; 6] = [0, 1, 2, 2, 3, 0];

pub struct Vertex {
    _pos: [f32; 3],
    _color: [f32; 3],
}

impl Vertex {
    pub fn get_binding_descriptions() -> vk::VertexInputBindingDescriptionBuilder<'static> {
        vk::VertexInputBindingDescriptionBuilder::new()
            .binding(0)
            .stride(size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescriptionBuilder<'static>; 2]
    {
        [
            // position
            vk::VertexInputAttributeDescriptionBuilder::new()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0),
            // color
            vk::VertexInputAttributeDescriptionBuilder::new()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                // offset of color is just size_of(pos)
                .offset(size_of::<[f32; 3]>() as u32),
        ]
    }
}

#[repr(C, align(16))]
pub struct UniformBufferObject {
    pub model: Mat4,
    pub view: Mat4,
    pub proj: Mat4,
}

impl UniformBufferObject {
    pub fn new() -> UniformBufferObject {
        UniformBufferObject {
            model: Mat4::identity(),
            view: Mat4::identity(),
            proj: Mat4::identity(),
        }
    }
}
