use crate::application::Opt;

use erupt::{vk, cstr, utils::surface};
use erupt::{InstanceLoader, DefaultEntryLoader, DeviceLoader};
use erupt::vk::{Queue, SurfaceKHR};

use winit::window::Window;

use std::os::raw::c_char;
use std::ffi::{CStr, CString, c_void};

use structopt::StructOpt;


pub const LAYER_KHRONOS_VALIDATION: *const c_char = cstr!("VK_LAYER_KHRONOS_validation");

pub fn create_instance(window: &Window, entry: &DefaultEntryLoader) -> InstanceLoader {
    // cmd arguments
    let opt = Opt::from_args();

    let application_name = CString::new("WIP").unwrap();
    let engine_name = CString::new("No Engine").unwrap();

    // generic application infomation
    let application_info = vk::ApplicationInfoBuilder::new()
        .application_name(&application_name)
        .application_version(vk::make_version(1, 0, 0))
        .engine_name(&engine_name)
        .engine_version(vk::make_version(1, 0, 0));

    // instance extensions required by winit surface
    let mut instance_extensions = surface::enumerate_required_extensions(window).unwrap();

    // check for -v --validation flags and enable/disable validation layers
    if opt.validation {
        // extension for debug callback
        instance_extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION_NAME);
    }

    // instance layers (pretty much just validation layers)
    let mut instance_layers = Vec::new();
    if opt.validation {
        // standard validation layer
        instance_layers.push(LAYER_KHRONOS_VALIDATION);
    }

    // bundling all of the previous
    let instance_info = vk::InstanceCreateInfoBuilder::new()
        .application_info(&application_info)
        .enabled_extension_names(&instance_extensions)
        .enabled_layer_names(&instance_layers);

    // create the instance :)
    match InstanceLoader::new(&entry, &instance_info, None) {
        Ok(instance) => instance,
        Err(e) => panic!("Le Instance creation failed! {:?}", e),
    }
}

pub fn setup_debug_messenger(instance: &InstanceLoader) -> vk::DebugUtilsMessengerEXT {
    // cmd args
    let opt = Opt::from_args();

    if opt.validation {
        let messenger_info = vk::DebugUtilsMessengerCreateInfoEXTBuilder::new()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING_EXT
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR_EXT,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL_EXT
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION_EXT
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE_EXT,
            )
            .pfn_user_callback(Some(debug_callback));

        unsafe { instance.create_debug_utils_messenger_ext(&messenger_info, None, None) }.unwrap()
    } else {
        // fallback to default callback if validation layers arn't on
        //we don't care about custom callback if validation layers arn't on
        Default::default()
    }
}

pub fn pick_physical_device_and_queue_family(
    instance: &InstanceLoader,
    surface: &SurfaceKHR,
    device_extensions: &Vec<*const i8>,
) -> (
    vk::PhysicalDevice,
    u32,
    vk::SurfaceFormatKHR,
    vk::PresentModeKHR,
    vk::PhysicalDeviceProperties,
) {
    unsafe { instance.enumerate_physical_devices(None) }
        .unwrap()
        .into_iter()

        // filter and map physical devices (& other stuff) for use with .max_with_key
        .filter_map(|physical_device| unsafe {
            let queue_family = match instance.get_physical_device_queue_family_properties(physical_device, None)
            .into_iter()
            .enumerate()
            .position(|(index, queue_family_properties)| {

                // need graphics flags for rendering & presentation
                queue_family_properties.queue_flags.contains(vk::QueueFlags::GRAPHICS)

                // need support for surface for le window
                && instance.get_physical_device_surface_support_khr(physical_device, index as u32, *surface, None).unwrap() 
                == true}) {
                    Some(queue_family) => queue_family as u32,
                    None => return None,
                };

            // get all formats supported by device
            let formats = instance.get_physical_device_surface_formats_khr(physical_device, *surface, None).unwrap();

            // prefer 32bit srgba
            let format = match formats.iter().find(|surface_format| { 
                surface_format.format == vk::Format::B8G8R8A8_SRGB
                && surface_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR_KHR

            // worst case fall back to first supported format
            }).or_else(|| formats.get(0)) {
                Some(surface_format) => surface_format.clone(),
                None => return None,
            };

            let present_mode = instance.get_physical_device_surface_present_modes_khr(physical_device, *surface, None)
            // prefer vsync
                .unwrap().into_iter().find(|present_mode| present_mode == &vk::PresentModeKHR::MAILBOX_KHR)
                // FIFO as fallback
                .unwrap_or(vk::PresentModeKHR::FIFO_KHR);

            // get supported device extensions
            let supported_device_extensions = instance.enumerate_device_extension_properties(physical_device, None, None).unwrap();

            // check for lack of support for all the required device extensions
            if !device_extensions.iter().all(|device_extension| {

                // dereference pointer to get extension
                let device_extension = CStr::from_ptr(*device_extension);

                // check if such extension is supported on device
                supported_device_extensions.iter().any(|properties| { 
                    CStr::from_ptr(properties.extension_name.as_ptr()) == device_extension
                })
            }) {
                return None;
            }

            // get physical device properties
            let device_properties = instance.get_physical_device_properties(physical_device, None);

            // return info for physical device
            Some((physical_device, queue_family, format, present_mode, device_properties))
        })
        .max_by_key(|(_, _, _, _, properties)| match properties.device_type {

            // prefer discrete gpu but settle for integrated
            vk::PhysicalDeviceType::DISCRETE_GPU => 2,
            vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
            _ => 0,
    }).expect("Big sad no supported physical devices found :(")
}

pub fn get_logical_device_and_queue(instance: &InstanceLoader, physical_device: vk::PhysicalDevice, device_extensions: &Vec<*const i8>, device_layers: &Vec<*const i8>, queue_family: u32) -> (DeviceLoader, Queue) {
    let queue_infos = vec![vk::DeviceQueueCreateInfoBuilder::new().queue_family_index(queue_family).queue_priorities(&[1.0])];

    let features = vk::PhysicalDeviceFeaturesBuilder::new();

    // create device info with features queried with pick physical device 
    let device_info = vk::DeviceCreateInfoBuilder::new().queue_create_infos(&queue_infos)
        .enabled_features(&features).enabled_extension_names(device_extensions).enabled_layer_names(device_layers);

    let device = DeviceLoader::new(&instance, physical_device, &device_info, None).unwrap();
    let queue = unsafe { device.get_device_queue(queue_family, 0, None)};

    (device, queue)
}

// debug callback signature
unsafe extern "system" fn debug_callback(
    _message_severity: vk::DebugUtilsMessageSeverityFlagBitsEXT,
    _message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    // print to stderr not stdout
    eprintln!(
        "{}",
        CStr::from_ptr((*p_callback_data).p_message).to_string_lossy()
    );

    vk::FALSE
}