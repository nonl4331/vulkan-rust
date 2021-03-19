use crate::application::{buffer, render};
use crate::Application;

use erupt::vk;

pub fn main_events_cleared(app: &mut Application) {
    // wait for image at current index to finish render to avoid submiting more than gpu can handle
    // u64::MAX disables cooldown
    unsafe {
        app.device
            .wait_for_fences(&[app.in_flight_fences[app.current_frame]], true, u64::MAX)
    }
    .expect("Failed on waiting for in_flight_fences[current_frame]!");

    // get index of next image in swapchain & check for invalid swapchain
    let result = unsafe {
        app.device.acquire_next_image_khr(
            app.swapchain,
            u64::MAX,
            Some(app.image_available_semaphores[app.current_frame]),
            None,
            None,
        )
    };

    let image_index = match result.raw {
        vk::Result::SUCCESS | vk::Result::SUBOPTIMAL_KHR => {
            result.expect("Failed to unwrap swapchain image!")
        }
        vk::Result::ERROR_OUT_OF_DATE_KHR => {
            app.resize_window();
            return;
        }
        _ => {
            panic!("Failed to aquire swap chain image!");
        }
    };

    buffer::update_uniform_buffer(
        &app.device,
        &mut app.ubo,
        &app.start,
        &app.uniform_buffer_memory[image_index as usize],
    );

    // get fence for swapchain image use
    let image_in_flight = app.images_in_flight[image_index as usize];

    // check if image is in use and if so wait for image to become available
    if !image_in_flight.is_null() {
        unsafe {
            app.device
                .wait_for_fences(&[image_in_flight], true, u64::MAX)
        }
        .expect("Failed on wait for images_in_flight[image_index]!");
    }

    // mark swapchain image for use with current frame
    app.images_in_flight[image_index as usize] = app.in_flight_fences[app.current_frame];

    // semaphores for current frame
    let image_available_semaphore = vec![app.image_available_semaphores[app.current_frame]];
    let render_finished_semaphore = vec![app.render_finished_semaphores[app.current_frame]];

    // submit info takes &vec
    let command_buffer = vec![app.command_buffers[image_index as usize]];

    let submit_info = vk::SubmitInfoBuilder::new()
        .wait_semaphores(&image_available_semaphore)
        .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
        .command_buffers(&command_buffer)
        .signal_semaphores(&render_finished_semaphore);

    // submit queue + fence reset
    unsafe {
        let in_flight_fence = app.in_flight_fences[app.current_frame];
        app.device
            .reset_fences(&[in_flight_fence])
            .expect("failed on images_in_flight[current_frame] fence reset!");
        app.device
            .queue_submit(app.queue, &[submit_info], Some(in_flight_fence))
    }
    .expect("Failed main queue submition!");

    // present info takes &vec[]
    let swapchain = vec![app.swapchain];

    // present info tkaes &vec[]
    let image_index = vec![image_index];

    let present_info = vk::PresentInfoKHRBuilder::new()
        .wait_semaphores(&render_finished_semaphore)
        .swapchains(&swapchain)
        .image_indices(&image_index);

    // presentation
    let result = unsafe { app.device.queue_present_khr(app.queue, &present_info) };

    if app.resized {
        app.resize_window();
        return;
    } else {
        match result.raw {
            vk::Result::SUCCESS => result.expect("Failed to unwrap queue presentation!"),
            vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR => {
                app.resize_window();
                return;
            }
            _ => {
                panic!("Failed to present swap chain image!")
            }
        }
    }

    // change current_frame to next frame
    app.current_frame = (app.current_frame + 1) % render::MAX_FRAMES_IN_FLIGHT;
}

pub fn loop_destroyed(app: &mut Application) {
    unsafe {
        // don't destroy in a non idle state
        app.device
            .device_wait_idle()
            .expect("Device wait idle failed on resize window!");

        // destroys objects that need change with window resize
        app.destroy_swapchain_related_objects();

        app.device
            .destroy_descriptor_set_layout(Some(app.descriptor_set_layout), None);

        app.device.destroy_buffer(Some(app.index_buffer), None);
        app.device.free_memory(Some(app.index_buffer_memory), None);

        app.device.destroy_buffer(Some(app.vertex_buffer), None);
        app.device.free_memory(Some(app.vertex_buffer_memory), None);

        // destroy all semaphores
        for &semaphore in app
            .image_available_semaphores
            .iter()
            .chain(app.render_finished_semaphores.iter())
        {
            app.device.destroy_semaphore(Some(semaphore), None);
        }

        // destroy fences (remember in_flight_fences[index_index] = frames_in_flight[current_frame])
        for &fence in &app.in_flight_fences {
            app.device.destroy_fence(Some(fence), None);
        }

        app.device
            .destroy_command_pool(Some(app.command_pool), None);

        app.device
            .destroy_shader_module(Some(app.shader_vert), None);
        app.device
            .destroy_shader_module(Some(app.shader_frag), None);

        app.device.destroy_device(None);

        app.instance.destroy_surface_khr(Some(app.surface), None);

        // messenger descruction
        if !app.messenger.is_null() {
            app.instance
                .destroy_debug_utils_messenger_ext(Some(app.messenger), None);
        }

        app.instance.destroy_instance(None);

        println!("All cleaned up!")
    }
}
