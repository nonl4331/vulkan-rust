use core::ffi::c_void;
use erupt::{vk, DeviceLoader, InstanceLoader};

use std::mem::size_of;

pub const VERTICES: [Vertex; 3] = [
    Vertex {
        pos: [0.0, -0.5],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        pos: [0.5, 0.5],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        pos: [-0.5, 0.5],
        color: [0.0, 0.0, 1.0],
    },
];

pub struct Vertex {
    pos: [f32; 2],
    color: [f32; 3],
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
                .offset(size_of::<[f32; 2]>() as u32),
        ]
    }
}

pub fn create_vertex_buffer(
    instance: &InstanceLoader,
    device: &DeviceLoader,
    physical_device: &vk::PhysicalDevice,
) -> (vk::Buffer, vk::DeviceMemory) {
    // pretty normal buffer is of size VERTICES as is used as a vertex buffer
    let buffer_info = vk::BufferCreateInfoBuilder::new()
        .size((size_of::<Vertex>() * VERTICES.len()) as u64)
        .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = unsafe { device.create_buffer(&buffer_info, None, None) }.unwrap();

    let buffer_memory = allocate_vertex_buffer(instance, device, physical_device, &buffer);

    (buffer, buffer_memory)
}

fn find_physical_device_memory(
    instance: &InstanceLoader,
    physical_device: &vk::PhysicalDevice,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> u32 {
    // get physical device memory properties
    let memory_properties =
        unsafe { instance.get_physical_device_memory_properties(*physical_device, None) };

    // get memory that aligns with type_filter & properties
    (0..memory_properties.memory_type_count)
        .find(|&i| {
            ((type_filter & (1 << i)) != 0)
                && ((memory_properties.memory_types[i as usize].property_flags & properties)
                    == properties)
        })
        .expect("Failed to find valid memory for vertex buffer allocation!")
}

pub fn allocate_vertex_buffer(
    instance: &InstanceLoader,
    device: &DeviceLoader,
    physical_device: &vk::PhysicalDevice,
    buffer: &vk::Buffer,
) -> vk::DeviceMemory {
    // get vertex buffer memory requirements
    let memory_requirements = unsafe { device.get_buffer_memory_requirements(*buffer, None) };

    let properties: vk::MemoryPropertyFlags =
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;

    // start to allocate vertex buffer
    let allocation_info = vk::MemoryAllocateInfoBuilder::new()
        .allocation_size(memory_requirements.size)
        .memory_type_index(find_physical_device_memory(
            instance,
            physical_device,
            memory_requirements.memory_type_bits,
            properties,
        ));

    // allocate vertex buffer memory
    let buffer_memory = unsafe { device.allocate_memory(&allocation_info, None, None) }
        .expect("Failed to allocate vertex buffer memory!");

    // bind such memory with buffer
    unsafe { device.bind_buffer_memory(*buffer, buffer_memory, 0) }
        .expect("Failed to bind vertex buffer memory");

    // copy vertex data to buffer
    unsafe {
        let mut data: *mut c_void = core::ptr::null_mut();

        // map physical_device memory to *data so we can copy onto it
        device
            .map_memory(
                buffer_memory,
                0,
                (size_of::<Vertex>() * VERTICES.len()) as u64,
                None,
                &mut data,
            )
            .expect("Failed to map memory for vertex buffer!");

        // copy over vertex data to buffer
        core::ptr::copy(
            &VERTICES,
            data as *mut [Vertex; 3],
            size_of::<Vertex>() * VERTICES.len(),
        );

        // unmap physical_device memory as we have copied the needed data over
        device.unmap_memory(buffer_memory);
    }

    buffer_memory
}
