use erupt::vk;
use erupt::vk::{Framebuffer, ImageView, SurfaceCapabilitiesKHR};
use erupt::DeviceLoader;

use crate::application::model::VERTICES;

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub fn create_framebuffers(
    device: &DeviceLoader,
    image_views: &Vec<ImageView>,
    render_pass: &vk::RenderPass,
    surface_capabilities: &SurfaceCapabilitiesKHR,
) -> Vec<Framebuffer> {
    // create framebuffers from single image views
    let framebuffers: Vec<_> = image_views
        .into_iter()
        .map(|view| {
            let attachments = vec![*view];
            let framebuffer_info = vk::FramebufferCreateInfoBuilder::new()
                .render_pass(*render_pass)
                .attachments(&attachments)
                .width(surface_capabilities.current_extent.width)
                .height(surface_capabilities.current_extent.height)
                .layers(1);
            unsafe { device.create_framebuffer(&framebuffer_info, None, None) }.unwrap()
        })
        .collect();

    // cannot return directly from function due to type inference?
    framebuffers
}

pub fn create_command_pool(device: &DeviceLoader, queue_family: u32) -> vk::CommandPool {
    // command pool for main graphics queue family
    let command_pool_info =
        vk::CommandPoolCreateInfoBuilder::new().queue_family_index(queue_family);

    unsafe { device.create_command_pool(&command_pool_info, None, None) }.unwrap()
}

pub fn allocate_command_buffers(
    device: &DeviceLoader,
    command_pool: &vk::CommandPool,
    framebuffers: &Vec<Framebuffer>,
) -> Vec<vk::CommandBuffer> {
    let command_buffer_allocation_info = vk::CommandBufferAllocateInfoBuilder::new()
        .command_pool(*command_pool)
        .command_buffer_count(framebuffers.len() as u32);

    unsafe { device.allocate_command_buffers(&command_buffer_allocation_info) }.unwrap()
}

pub fn record_command_buffers(
    device: &DeviceLoader,
    pipeline: &vk::Pipeline,
    command_buffers: &Vec<vk::CommandBuffer>,
    framebuffers: &Vec<Framebuffer>,
    render_pass: &vk::RenderPass,
    surface_capabilities: &SurfaceCapabilitiesKHR,
    vertex_buffer: &vk::Buffer,
) {
    // command buffer for each framebuffer
    for (command_buffer, framebuffer) in command_buffers.iter().zip(framebuffers.iter()) {
        let command_buffer_begin_info = vk::CommandBufferBeginInfoBuilder::new();

        unsafe { device.begin_command_buffer(*command_buffer, &command_buffer_begin_info) }
            .unwrap();

        // greenish clear color cause black is boring
        let clear_color = vk::ClearColorValue {
            float32: [0.1961, 0.6588, 0.3216, 1.0],
        };

        //  only one attachment (vk::ImageView) in framebuffer
        let clear_colors = vec![vk::ClearValue { color: clear_color }];

        // do render on the entire screen
        let screen_size = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: surface_capabilities.current_extent,
        };

        let render_pass_begin_info = vk::RenderPassBeginInfoBuilder::new()
            .render_pass(*render_pass)
            .framebuffer(*framebuffer)
            .render_area(screen_size)
            .clear_values(&clear_colors);

        // updated viewport & scissor
        let viewports = vec![vk::ViewportBuilder::new()
            .x(0.0)
            .y(0.0)
            .width(surface_capabilities.current_extent.width as f32)
            .height(surface_capabilities.current_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)];

        let scissors = vec![vk::Rect2DBuilder::new()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(surface_capabilities.current_extent)];

        // render triangle (which is stored in vertex shader code)
        unsafe {
            device.cmd_begin_render_pass(
                *command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

            device.cmd_bind_pipeline(*command_buffer, vk::PipelineBindPoint::GRAPHICS, *pipeline);

            // set vertex buffer
            let vertex_buffers = &[*vertex_buffer];
            let offsets = [0];
            device.cmd_bind_vertex_buffers(*command_buffer, 0, vertex_buffers, &offsets);

            // set viewport & scissors (incase of resize)
            device.cmd_set_viewport(*command_buffer, 0, &viewports);
            device.cmd_set_scissor(*command_buffer, 0, &scissors);

            device.cmd_draw(*command_buffer, VERTICES.len() as u32, 1, 0, 0);
            device.cmd_end_render_pass(*command_buffer);

            device.end_command_buffer(*command_buffer).unwrap()
        }
    }
}

pub fn create_sync_primitives(
    device: &DeviceLoader,
    swapchain_size: usize,
) -> (
    Vec<vk::Semaphore>,
    Vec<vk::Semaphore>,
    Vec<vk::Fence>,
    Vec<vk::Fence>,
) {
    let semaphore_info = vk::SemaphoreCreateInfoBuilder::new();
    let fence_info = vk::FenceCreateInfoBuilder::new().flags(vk::FenceCreateFlags::SIGNALED);

    let image_available_semaphore: Vec<_> = (0..MAX_FRAMES_IN_FLIGHT)
        .map(|_| unsafe { device.create_semaphore(&semaphore_info, None, None) }.unwrap())
        .collect();

    let render_finished_semaphore: Vec<_> = (0..MAX_FRAMES_IN_FLIGHT)
        .map(|_| unsafe { device.create_semaphore(&semaphore_info, None, None) }.unwrap())
        .collect();

    let in_flight_fences = (0..MAX_FRAMES_IN_FLIGHT)
        .map(|_| unsafe { device.create_fence(&fence_info, None, None) }.unwrap())
        .collect();

    let images_in_flight = (0..swapchain_size).map(|_| vk::Fence::null()).collect();

    (
        image_available_semaphore,
        render_finished_semaphore,
        in_flight_fences,
        images_in_flight,
    )
}
