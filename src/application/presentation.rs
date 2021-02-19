use erupt::vk;
use erupt::vk::{Image, ImageView, SurfaceCapabilitiesKHR, SwapchainKHR};
use erupt::{DeviceLoader, InstanceLoader};

use std::cmp::{max, min};

pub fn create_swapchain_and_images(
    instance: &InstanceLoader,
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
    format: vk::SurfaceFormatKHR,
    present_mode: vk::PresentModeKHR,
    device: &DeviceLoader,
) -> (SwapchainKHR, Vec<Image>, SurfaceCapabilitiesKHR) {
    // get surface capabilities
    let surface_capabilities = unsafe {
        instance.get_physical_device_surface_capabilities_khr(physical_device, surface, None)
    }
    .unwrap();

    // min + 1 to prevent stalling by the driver because of availibility
    let mut image_count = surface_capabilities.min_image_count + 1;

    // prevent image_count from being 0 or larger than max capabilites of the surface
    image_count = min(image_count, max(surface_capabilities.max_image_count, 1));

    let swapchain_info = vk::SwapchainCreateInfoKHRBuilder::new()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(format.format)
        .image_color_space(format.color_space)
        .image_extent(surface_capabilities.current_extent)
        .image_array_layers(1)
        // image is owned by single queue (for now)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(surface_capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagBitsKHR::OPAQUE_KHR)
        .present_mode(present_mode)
        .clipped(true)
        .old_swapchain(vk::SwapchainKHR::null());

    let swapchain = unsafe { device.create_swapchain_khr(&swapchain_info, None, None) }.unwrap();
    let swapchain_images = unsafe { device.get_swapchain_images_khr(swapchain, None) }.unwrap();

    (swapchain, swapchain_images, surface_capabilities)
}

pub fn get_image_views(
    swapchain_images: &Vec<Image>,
    device: &DeviceLoader,
    format: vk::SurfaceFormatKHR,
) -> Vec<ImageView> {
    // don't remap components
    let component_mapping = vk::ComponentMapping {
        r: vk::ComponentSwizzle::IDENTITY,
        g: vk::ComponentSwizzle::IDENTITY,
        b: vk::ComponentSwizzle::IDENTITY,
        a: vk::ComponentSwizzle::IDENTITY,
    };

    let subresource_range = vk::ImageSubresourceRangeBuilder::new()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        // no additional mip maps for now
        .level_count(1)
        .base_array_layer(0)
        .layer_count(1)
        .build();

    // specify Vec<_> for .collect()
    let swapchain_image_views: Vec<_> = swapchain_images
        .iter()
        .map(|image| {
            let image_view_info = vk::ImageViewCreateInfoBuilder::new()
                .image(*image)
                .view_type(vk::ImageViewType::_2D)
                .format(format.format)
                .components(component_mapping)
                .subresource_range(subresource_range);

            unsafe { device.create_image_view(&image_view_info, None, None) }.unwrap()
        })
        .collect();

    swapchain_image_views
}
