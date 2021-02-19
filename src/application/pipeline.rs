use std::os::raw::c_char;

use erupt::{cstr, utils, vk, DeviceLoader};

use std::ffi::CStr;

// shader spvs
pub const SHADER_VERT: &[u8] = include_bytes!("../../res/shaders/vert.spv");
pub const SHADER_FRAG: &[u8] = include_bytes!("../../res/shaders/frag.spv");

pub const SHADER_ENTRY: *const c_char = cstr!("main");

fn create_shader_stages<'a>(
    device: &DeviceLoader,
) -> Vec<vk::PipelineShaderStageCreateInfoBuilder<'a>> {
    // vertex shader
    let vert_decoded = utils::decode_spv(SHADER_VERT).unwrap();
    let module_info = vk::ShaderModuleCreateInfoBuilder::new().code(&vert_decoded);
    let shader_vert = unsafe { device.create_shader_module(&module_info, None, None) }.unwrap();

    // fragment shader
    let frag_decoded = utils::decode_spv(SHADER_FRAG).unwrap();
    let module_info = vk::ShaderModuleCreateInfoBuilder::new().code(&frag_decoded);
    let shader_frag = unsafe { device.create_shader_module(&module_info, None, None) }.unwrap();

    let shader_stages = vec![vk::PipelineShaderStageCreateInfoBuilder::new()
        .stage(vk::ShaderStageFlagBits::VERTEX)
        .module(shader_vert)
        .name(unsafe { CStr::from_ptr(SHADER_ENTRY) })
        .stage(vk::ShaderStageFlagBits::FRAGMENT)
        .module(shader_frag)
        .name(unsafe { CStr::from_ptr(SHADER_ENTRY) })];

    shader_stages
}
