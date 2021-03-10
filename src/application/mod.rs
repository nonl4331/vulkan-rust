// instance, surface, devices, queues
mod setup;

// swapchain, image views
mod presentation;

// graphics pipeline
mod pipeline;

// rendering & presentation
mod render;

// model loading
mod model;

// buffers
mod buffer;

use crate::application::setup::LAYER_KHRONOS_VALIDATION;
use winit::dpi::PhysicalSize;

use erupt::vk;
use erupt::vk::{Image, ImageView, SurfaceCapabilitiesKHR, SurfaceKHR, SwapchainKHR};
use erupt::{utils::surface, DefaultEntryLoader, DeviceLoader, InstanceLoader};

use std::ffi::CStr;

use winit::{
    event::{
        DeviceEvent, ElementState, Event, KeyboardInput, StartCause, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use structopt::StructOpt;

// struct for cmd arguments
#[derive(Debug, StructOpt)]
pub struct Opt {
    #[structopt(short, long)]
    validation: bool,
}

// Application struct
pub struct Application {
    event_loop: Option<winit::event_loop::EventLoop<()>>,
    pub window: Window,
    pub messenger: vk::DebugUtilsMessengerEXT,
    pub surface: SurfaceKHR,
    pub device_extensions: Vec<*const i8>,
    pub physical_device: vk::PhysicalDevice,
    pub queue_family: u32,
    pub surface_format: vk::SurfaceFormatKHR,
    pub present_mode: vk::PresentModeKHR,
    pub physical_device_properties: vk::PhysicalDeviceProperties,
    pub device: DeviceLoader,
    pub queue: vk::Queue,
    pub swapchain: SwapchainKHR,
    pub swapchain_images: Vec<Image>,
    pub surface_capabilities: SurfaceCapabilitiesKHR,
    pub swapchain_image_views: Vec<ImageView>,
    pub shader_vert: vk::ShaderModule,
    pub shader_frag: vk::ShaderModule,
    pub render_pass: vk::RenderPass,
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub command_pool: vk::CommandPool,
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    pub command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    images_in_flight: Vec<vk::Fence>,
    current_frame: usize,
    resized: bool,

    // instance loader is at bottom due to drop order
    pub instance: InstanceLoader,
}

// Main and only impl block
impl Application {
    pub fn new(entry: &DefaultEntryLoader) -> Application {
        // cmd arguments
        let opt = Opt::from_args();

        // from winit
        let event_loop = EventLoop::new();

        // resizable false for now
        let window = match WindowBuilder::new()
            .with_title("WIP")
            .with_resizable(false)
            .build(&event_loop)
        {
            Ok(window) => window,
            Err(e) => panic!("Le Window creation failed! {:?}", e),
        };

        let mut instance = setup::create_instance(&window, &entry);

        let messenger = setup::setup_debug_messenger(&instance);

        let surface = unsafe { surface::create_surface(&mut instance, &window, None) }
            .expect("Failed to create surface!");

        // needed extension for presention
        let device_extensions = vec![vk::KHR_SWAPCHAIN_EXTENSION_NAME];

        // get physical device & queue family
        let (
            physical_device,
            queue_family,
            surface_format,
            present_mode,
            physical_device_properties,
        ) = setup::pick_physical_device_and_queue_family(&instance, &surface, &device_extensions);

        // get device layers (pretty much just validation)
        let mut device_layers = Vec::new();
        if opt.validation {
            device_layers.push(LAYER_KHRONOS_VALIDATION);
        }

        // get queue & logical device
        let (device, queue) = setup::get_logical_device_and_queue(
            &instance,
            physical_device,
            &device_extensions,
            &device_layers,
            queue_family,
        );

        println!("Using physical device - {:?}", unsafe {
            CStr::from_ptr(physical_device_properties.device_name.as_ptr())
        });

        // create swapchain and get image references
        let (swapchain, swapchain_images, surface_capabilities) =
            presentation::create_swapchain_and_images(
                &instance,
                physical_device,
                surface,
                surface_format,
                present_mode,
                &device,
            );

        // get swapchain image views
        let swapchain_image_views =
            presentation::get_image_views(&swapchain_images, &device, surface_format);

        let (shader_vert, shader_frag) = pipeline::create_shader_modules(&device);

        // graphics pipeline & render pass
        let (pipeline, pipeline_layout, render_pass) =
            pipeline::create_graphics_pipeline(&device, shader_vert, shader_frag, surface_format);

        // create framebuffers
        let framebuffers = render::create_framebuffers(
            &device,
            &swapchain_image_views,
            &render_pass,
            &surface_capabilities,
        );

        // create command pool
        let command_pool = render::create_command_pool(&device, queue_family);

        // create vertex_buffer
        let (vertex_buffer, vertex_buffer_memory) = buffer::create_vertex_buffer(
            &instance,
            &device,
            &physical_device,
            &command_pool,
            &queue,
        );

        // create index_buffer
        let (index_buffer, index_buffer_memory) = buffer::create_index_buffer(
            &instance,
            &device,
            &physical_device,
            &command_pool,
            &queue,
        );

        // allocate command buffers
        let command_buffers =
            render::allocate_command_buffers(&device, &command_pool, &framebuffers);

        // record command buffers
        render::record_command_buffers(
            &device,
            &pipeline,
            &command_buffers,
            &framebuffers,
            &render_pass,
            &surface_capabilities,
            &vertex_buffer,
            &index_buffer,
        );

        // create semaphores & fences
        let (
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight,
        ) = render::create_sync_primitives(&device, swapchain_images.len());

        let (current_frame, resized) = (0, false);

        // Struct creation
        Application {
            event_loop: Some(event_loop),
            window,
            instance,
            messenger,
            surface,
            device_extensions,
            physical_device,
            queue_family,
            surface_format,
            present_mode,
            physical_device_properties,
            device,
            queue,
            swapchain,
            swapchain_images,
            surface_capabilities,
            swapchain_image_views,
            shader_vert,
            shader_frag,
            render_pass,
            pipeline_layout,
            pipeline,
            framebuffers,
            command_pool,
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer,
            index_buffer_memory,
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight,
            current_frame,
            resized,
        }
    }

    fn destroy_swapchain_related_objects(&self) {
        unsafe {
            // destory framebuffers
            for &framebuffer in &self.framebuffers {
                self.device.destroy_framebuffer(Some(framebuffer), None);
            }

            // destroy command buffers
            self.device
                .free_command_buffers(self.command_pool, &self.command_buffers);

            // graphics pipeline destruction
            self.device.destroy_pipeline(Some(self.pipeline), None);

            // render pass destruction
            self.device
                .destroy_render_pass(Some(self.render_pass), None);

            // graphics pipeline layout destruction
            self.device
                .destroy_pipeline_layout(Some(self.pipeline_layout), None);

            // image view destruction
            for &image_view in &self.swapchain_image_views {
                self.device.destroy_image_view(Some(image_view), None);
            }

            // swapchain destruction
            self.device
                .destroy_swapchain_khr(Some(self.swapchain), None);
        }
    }

    fn resize_window(&mut self) {
        unsafe {
            // don't resize in a non idle state
            self.device
                .device_wait_idle()
                .expect("Device wait idle failed on resize window!");

            // cleanup swapchain and related devices
            self.destroy_swapchain_related_objects();

            // create swapchain and get image references
            let (swapchain, swapchain_images, surface_capabilities) =
                presentation::create_swapchain_and_images(
                    &self.instance,
                    self.physical_device,
                    self.surface,
                    self.surface_format,
                    self.present_mode,
                    &self.device,
                );

            // get swapchain image views
            let swapchain_image_views =
                presentation::get_image_views(&swapchain_images, &self.device, self.surface_format);

            // graphics pipeline & render pass
            let (pipeline, pipeline_layout, render_pass) = pipeline::create_graphics_pipeline(
                &self.device,
                self.shader_vert,
                self.shader_frag,
                self.surface_format,
            );

            // create framebuffers
            let framebuffers = render::create_framebuffers(
                &self.device,
                &swapchain_image_views,
                &render_pass,
                &surface_capabilities,
            );

            // allocate command buffers
            let command_buffers =
                render::allocate_command_buffers(&self.device, &self.command_pool, &framebuffers);

            // record command buffers
            render::record_command_buffers(
                &self.device,
                &pipeline,
                &command_buffers,
                &framebuffers,
                &render_pass,
                &surface_capabilities,
                &self.vertex_buffer,
                &self.index_buffer,
            );

            self.swapchain = swapchain;
            self.swapchain_images = swapchain_images;
            self.surface_capabilities = surface_capabilities;
            self.swapchain_image_views = swapchain_image_views;
            self.pipeline = pipeline;
            self.pipeline_layout = pipeline_layout;
            self.render_pass = render_pass;
            self.framebuffers = framebuffers;
            self.command_buffers = command_buffers;
            self.resized = false;
        };
    }

    pub fn run(mut self) -> ! {
        let event_loop = self
            .event_loop
            .take()
            .expect("Failed to take event loop out of Option!");
        event_loop.run(move |event, _, control_flow| match event {
            // Init
            Event::NewEvents(StartCause::Init) => {
                *control_flow = ControlFlow::Poll;
            }

            Event::MainEventsCleared => {
                // wait for image at current index to finish render to avoid submiting more than gpu can handle
                // u64::MAX disables cooldown
                unsafe {
                    self.device.wait_for_fences(
                        &[self.in_flight_fences[self.current_frame]],
                        true,
                        u64::MAX,
                    )
                }
                .expect("Failed on waiting for in_flight_fences[current_frame]!");

                // get index of next image in swapchain & check for invalid swapchain
                let result = unsafe {
                    self.device.acquire_next_image_khr(
                        self.swapchain,
                        u64::MAX,
                        Some(self.image_available_semaphores[self.current_frame]),
                        None,
                        None,
                    )
                };

                let image_index = match result.raw {
                    vk::Result::SUCCESS | vk::Result::SUBOPTIMAL_KHR => {
                        result.expect("Failed to unwrap swapchain image!")
                    }
                    vk::Result::ERROR_OUT_OF_DATE_KHR => {
                        self.resize_window();
                        return;
                    }
                    _ => {
                        panic!("Failed to aquire swap chain image!");
                    }
                };

                // get fence for swapchain image use
                let image_in_flight = self.images_in_flight[image_index as usize];

                // check if image is in use and if so wait for image to become available
                if !image_in_flight.is_null() {
                    unsafe {
                        self.device
                            .wait_for_fences(&[image_in_flight], true, u64::MAX)
                    }
                    .expect("Failed on wait for images_in_flight[image_index]!");
                }

                // mark swapchain image for use with current frame
                self.images_in_flight[image_index as usize] =
                    self.in_flight_fences[self.current_frame];

                // semaphores for current frame
                let image_available_semaphore =
                    vec![self.image_available_semaphores[self.current_frame]];
                let render_finished_semaphore =
                    vec![self.render_finished_semaphores[self.current_frame]];

                // submit info takes &vec
                let command_buffer = vec![self.command_buffers[image_index as usize]];

                let submit_info = vk::SubmitInfoBuilder::new()
                    .wait_semaphores(&image_available_semaphore)
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                    .command_buffers(&command_buffer)
                    .signal_semaphores(&render_finished_semaphore);

                // submit queue + fence reset
                unsafe {
                    let in_flight_fence = self.in_flight_fences[self.current_frame];
                    self.device
                        .reset_fences(&[in_flight_fence])
                        .expect("failed on images_in_flight[current_frame] fence reset!");
                    self.device
                        .queue_submit(self.queue, &[submit_info], Some(in_flight_fence))
                }
                .expect("Failed main queue submition!");

                // present info takes &vec[]
                let swapchain = vec![self.swapchain];

                // present info tkaes &vec[]
                let image_index = vec![image_index];

                let present_info = vk::PresentInfoKHRBuilder::new()
                    .wait_semaphores(&render_finished_semaphore)
                    .swapchains(&swapchain)
                    .image_indices(&image_index);

                // presentation
                let result = unsafe { self.device.queue_present_khr(self.queue, &present_info) };

                if self.resized {
                    self.resize_window();
                    return;
                } else {
                    match result.raw {
                        vk::Result::SUCCESS => {
                            result.expect("Failed to unwrap queue presentation!")
                        }
                        vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR => {
                            self.resize_window();
                            return;
                        }
                        _ => {
                            panic!("Failed to present swap chain image!")
                        }
                    }
                }

                // get current_frame to next frame
                self.current_frame = (self.current_frame + 1) % render::MAX_FRAMES_IN_FLIGHT;
            }

            // Window events
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Resized { .. } => {
                    // halt on minimization?
                    if self.window.inner_size() == PhysicalSize::new(0, 0) {
                        *control_flow = ControlFlow::Wait;
                    }

                    self.resized = true;
                }
                _ => (),
            },

            // Input events
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::Key(KeyboardInput {
                    virtual_keycode: Some(scancode),
                    state,
                    ..
                }) => match (scancode, state) {
                    (VirtualKeyCode::Escape, ElementState::Released) => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => (),
                },
                _ => (),
            },

            // Loop destruction
            Event::LoopDestroyed => unsafe {
                // don't destroy in a non idle state
                self.device
                    .device_wait_idle()
                    .expect("Device wait idle failed on resize window!");

                // destroys objects that need change with window resize
                self.destroy_swapchain_related_objects();

                // destory index buffer and free memory
                self.device.destroy_buffer(Some(self.index_buffer), None);
                self.device
                    .free_memory(Some(self.index_buffer_memory), None);

                // destory vertex buffer and free memory
                self.device.destroy_buffer(Some(self.vertex_buffer), None);
                self.device
                    .free_memory(Some(self.vertex_buffer_memory), None);

                // destroy all semaphores
                for &semaphore in self
                    .image_available_semaphores
                    .iter()
                    .chain(self.render_finished_semaphores.iter())
                {
                    self.device.destroy_semaphore(Some(semaphore), None);
                }

                // destroy fences (remember in_flight_fences[index_index] = frames_in_flight[current_frame])
                for &fence in &self.in_flight_fences {
                    self.device.destroy_fence(Some(fence), None);
                }

                // destroy command_pool
                self.device
                    .destroy_command_pool(Some(self.command_pool), None);

                // destory shader modules
                self.device
                    .destroy_shader_module(Some(self.shader_vert), None);
                self.device
                    .destroy_shader_module(Some(self.shader_frag), None);

                // logical device destruction
                self.device.destroy_device(None);

                // surface destruction
                self.instance.destroy_surface_khr(Some(self.surface), None);

                // messenger descruction
                if !self.messenger.is_null() {
                    self.instance
                        .destroy_debug_utils_messenger_ext(Some(self.messenger), None);
                }

                // instance destruction
                self.instance.destroy_instance(None);
                println!("All cleaned up!")
            },

            _ => (),
        })
    }
}
