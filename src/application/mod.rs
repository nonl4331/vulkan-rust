// instance, surface, devices, queues
mod setup;

// swapchain, image views
mod presentation;

use crate::application::setup::LAYER_KHRONOS_VALIDATION;

use erupt::vk;
use erupt::vk::{Image, ImageView, SurfaceKHR, SwapchainKHR};
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
    pub event_loop: Option<winit::event_loop::EventLoop<()>>,
    pub window: Window,
    pub instance: InstanceLoader,
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
    pub swapchain_image_views: Vec<ImageView>,
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

        let surface = unsafe { surface::create_surface(&mut instance, &window, None) }.unwrap();

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
        let (swapchain, swapchain_images) = presentation::create_swapchain_and_images(
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
            swapchain_image_views,
        }
    }

    pub fn run(mut self) -> ! {
        let event_loop = self.event_loop.take().unwrap();
        event_loop.run(move |event, _, control_flow| match event {
            // Init
            Event::NewEvents(StartCause::Init) => {
                *control_flow = ControlFlow::Poll;
            }

            // Window events
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
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
                // wait till finished
                self.device.device_wait_idle().unwrap();

                // image view destruction
                for &image_view in &self.swapchain_image_views {
                    self.device.destroy_image_view(Some(image_view), None);
                }

                // swapchain destruction
                self.device
                    .destroy_swapchain_khr(Some(self.swapchain), None);

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
