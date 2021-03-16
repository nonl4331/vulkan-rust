use crate::application::model::{Index, UniformBufferObject, Vertex, INDICIES, VERTICES};
use core::ffi::c_void;
use erupt::{vk, DeviceLoader, InstanceLoader};

use std::time::Instant;

use std::mem::size_of;

pub fn create_buffer(
    instance: &InstanceLoader,
    physical_device: &vk::PhysicalDevice,
    device: &DeviceLoader,
    buffer_size: u64,
    usage: vk::BufferUsageFlags,
    sharing_mode: vk::SharingMode,
    properties: vk::MemoryPropertyFlags,
) -> (vk::Buffer, vk::DeviceMemory) {
    let buffer_info = vk::BufferCreateInfoBuilder::new()
        .size(buffer_size)
        .usage(usage)
        .sharing_mode(sharing_mode);

    // create buffer
    let buffer = unsafe { device.create_buffer(&buffer_info, None, None) }
        .expect("Failed to create buffer!");

    // get buffer memory requirements
    let memory_requirements = unsafe { device.get_buffer_memory_requirements(buffer, None) };

    // start to allocate buffer
    let allocation_info = vk::MemoryAllocateInfoBuilder::new()
        .allocation_size(memory_requirements.size)
        .memory_type_index(find_physical_device_memory(
            instance,
            physical_device,
            memory_requirements.memory_type_bits,
            properties,
        ));

    // allocate buffer memory
    let buffer_memory = unsafe { device.allocate_memory(&allocation_info, None, None) }
        .expect("Failed to allocate staging buffer memory!");

    // bind such memory with buffer
    unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0) }
        .expect("Failed to bind vertex staging memory");

    (buffer, buffer_memory)
}

pub fn create_vertex_buffer(
    instance: &InstanceLoader,
    device: &DeviceLoader,
    physical_device: &vk::PhysicalDevice,
    command_pool: &vk::CommandPool,
    queue: &vk::Queue,
) -> (vk::Buffer, vk::DeviceMemory) {
    let buffer_size = (size_of::<Vertex>() * VERTICES.len()) as u64;

    // create temp staging buffer
    let (staging_buffer, staging_buffer_memory) = create_buffer(
        instance,
        physical_device,
        device,
        buffer_size,
        // used as the source for the transfer from host visible memory
        // to (possibly) more optimized memory (that might not be host visible)
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::SharingMode::EXCLUSIVE,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );

    // copy vertex data to staging buffer
    copy_to_staging_buffer::<[Vertex; 4]>(
        instance,
        device,
        physical_device,
        &staging_buffer,
        &staging_buffer_memory,
        &VERTICES,
        VERTICES.len() * size_of::<Vertex>(),
    );

    // create real vertex buffer
    let (vertex_buffer, vertex_buffer_memory) = create_buffer(
        instance,
        physical_device,
        device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::SharingMode::EXCLUSIVE,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );

    // copy from host visible staging buffer
    // to device local vertex buffer
    copy_buffer(
        device,
        command_pool,
        queue,
        &staging_buffer,
        &vertex_buffer,
        buffer_size,
    );

    // clean up staging buffer & memory
    unsafe { device.destroy_buffer(Some(staging_buffer), None) };
    unsafe { device.free_memory(Some(staging_buffer_memory), None) };

    (vertex_buffer, vertex_buffer_memory)
}

pub fn create_index_buffer(
    instance: &InstanceLoader,
    device: &DeviceLoader,
    physical_device: &vk::PhysicalDevice,
    command_pool: &vk::CommandPool,
    queue: &vk::Queue,
) -> (vk::Buffer, vk::DeviceMemory) {
    let buffer_size = (size_of::<Index>() * INDICIES.len()) as u64;

    // create temp staging buffer
    let (staging_buffer, staging_buffer_memory) = create_buffer(
        instance,
        physical_device,
        device,
        buffer_size,
        // used as the source for the transfer from host visible memory
        // to (possibly) more optimized memory (that might not be host visible)
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::SharingMode::EXCLUSIVE,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );

    // copy index data to staging buffer
    copy_to_staging_buffer::<[Index; 6]>(
        instance,
        device,
        physical_device,
        &staging_buffer,
        &staging_buffer_memory,
        &INDICIES,
        INDICIES.len() * size_of::<Index>(),
    );

    // create real index buffer
    let (index_buffer, index_buffer_memory) = create_buffer(
        instance,
        physical_device,
        device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
        vk::SharingMode::EXCLUSIVE,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );

    // copy from host visible staging buffer
    // to device local index buffer
    copy_buffer(
        &device,
        &command_pool,
        &queue,
        &staging_buffer,
        &index_buffer,
        buffer_size,
    );

    // clean up staging buffer & memory
    unsafe { device.destroy_buffer(Some(staging_buffer), None) };
    unsafe { device.free_memory(Some(staging_buffer_memory), None) };

    (index_buffer, index_buffer_memory)
}

pub fn create_uniform_buffer(
    instance: &InstanceLoader,
    physical_device: &vk::PhysicalDevice,
    device: &DeviceLoader,
    swapchain_length: usize,
) -> (Vec<vk::Buffer>, Vec<vk::DeviceMemory>) {
    let buffer_size = size_of::<UniformBufferObject>() as u64;

    // create uniform buffer & memory for each image in swapchain
    (0..swapchain_length)
        .map(|_| {
            create_buffer(
                instance,
                physical_device,
                device,
                buffer_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::SharingMode::EXCLUSIVE,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )
        })
        .unzip()
}

pub fn update_uniform_buffer(
    device: &DeviceLoader,
    ubo: &mut UniformBufferObject,
    start: &Instant,
    uniform_buffer_memory: &vk::DeviceMemory,
) {
    let duration = Instant::now().duration_since(*start).as_secs_f32();

    ubo.model = ultraviolet::mat::Mat4::from_rotation_z(duration);

    // copy data to buffer
    unsafe {
        let mut data: *mut c_void = core::ptr::null_mut();

        // map physical_device memory to *data so we can copy onto it
        device
            .map_memory(
                *uniform_buffer_memory,
                0,
                size_of::<UniformBufferObject>() as u64,
                None,
                &mut data,
            )
            .expect("Failed to map memory for uniform buffer!");

        // copy over data to buffer
        core::ptr::copy(ubo, data as *mut UniformBufferObject, 1);

        // unmap physical_device memory as we have copied the needed data over
        device.unmap_memory(*uniform_buffer_memory);
    };
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

pub fn copy_to_staging_buffer<T>(
    _instance: &InstanceLoader,
    device: &DeviceLoader,
    _physical_device: &vk::PhysicalDevice,
    _buffer: &vk::Buffer,
    buffer_memory: &vk::DeviceMemory,
    buffer_data: &T,
    buffer_size: usize,
) {
    // copy data to buffer
    unsafe {
        let mut data: *mut c_void = core::ptr::null_mut();

        // map physical_device memory to *data so we can copy onto it
        device
            .map_memory(*buffer_memory, 0, buffer_size as u64, None, &mut data)
            .expect("Failed to map memory for staging buffer!");

        // copy over data to buffer
        core::ptr::copy(buffer_data, data as *mut T, 1);

        // unmap physical_device memory as we have copied the needed data over
        device.unmap_memory(*buffer_memory);
    };
}

fn copy_buffer(
    device: &DeviceLoader,
    command_pool: &vk::CommandPool,
    queue: &vk::Queue,
    src_buffer: &vk::Buffer,
    dst_buffer: &vk::Buffer,
    buffer_size: u64,
) {
    // allocate temp command buffer for copy operation
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfoBuilder::new()
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(*command_pool)
        .command_buffer_count(1);

    let command_buffer = unsafe { device.allocate_command_buffers(&command_buffer_allocate_info) }
        .expect("Failed to allocate copy command buffer!");

    let begin_info = vk::CommandBufferBeginInfoBuilder::new()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    // begin copy operation
    unsafe { device.begin_command_buffer(command_buffer[0], &begin_info) }
        .expect("Failed to begin recording copy command buffer!");

    let copy_region = vec![vk::BufferCopyBuilder::new().size(buffer_size)];

    unsafe { device.cmd_copy_buffer(command_buffer[0], *src_buffer, *dst_buffer, &copy_region) };

    unsafe { device.end_command_buffer(command_buffer[0]) }
        .expect("Failed to end recording copy command buffer!");

    let submit_info = vk::SubmitInfoBuilder::new().command_buffers(&command_buffer);

    // submit command buffer to queue
    unsafe { device.queue_submit(*queue, &[submit_info], None) }
        .expect("Failed to submit queue with copy command buffer!");

    // wait idle then free command buffer
    unsafe { device.queue_wait_idle(*queue) }
        .expect("Queue wait idle failed in copy buffer function!");

    unsafe { device.free_command_buffers(*command_pool, &command_buffer) };
}
