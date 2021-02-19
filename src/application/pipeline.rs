use std::os::raw::c_char;

use erupt::{cstr, utils, vk, vk::SurfaceCapabilitiesKHR, DeviceLoader};

use std::ffi::CStr;

// shader spvs
pub const SHADER_VERT: &[u8] = include_bytes!("../../res/shaders/vert.spv");
pub const SHADER_FRAG: &[u8] = include_bytes!("../../res/shaders/frag.spv");

pub const SHADER_ENTRY: *const c_char = cstr!("main");

pub fn create_shader_modules<'a>(device: &DeviceLoader) -> (vk::ShaderModule, vk::ShaderModule) {
    // vertex shader
    let vert_decoded = utils::decode_spv(SHADER_VERT).unwrap();
    let module_info = vk::ShaderModuleCreateInfoBuilder::new().code(&vert_decoded);
    let shader_vert = unsafe { device.create_shader_module(&module_info, None, None) }.unwrap();

    // fragment shader
    let frag_decoded = utils::decode_spv(SHADER_FRAG).unwrap();
    let module_info = vk::ShaderModuleCreateInfoBuilder::new().code(&frag_decoded);
    let shader_frag = unsafe { device.create_shader_module(&module_info, None, None) }.unwrap();

    (shader_vert, shader_frag)
}

// stuff for graphics pipeline bar render pass, pipeline layout (WIP) and shader stages
fn create_fixed_functions(
    surface_capabilities: SurfaceCapabilitiesKHR,
    device: &DeviceLoader,
) -> (
    vk::PipelineVertexInputStateCreateInfoBuilder<'_>,
    vk::PipelineInputAssemblyStateCreateInfoBuilder<'_>,
    Vec<vk::ViewportBuilder<'_>>,
    Vec<vk::Rect2DBuilder<'_>>,
    vk::PipelineRasterizationStateCreateInfoBuilder<'_>,
    vk::PipelineMultisampleStateCreateInfoBuilder<'_>,
    Vec<vk::PipelineColorBlendAttachmentStateBuilder<'_>>,
    vk::PipelineLayout,
) {
    // triangle is built into vertex shader so no input (for now)
    let vertex_input = vk::PipelineVertexInputStateCreateInfoBuilder::new();

    // how vertices are interpreted, TRIANGLE_LIST is just regular triangles not triangle strip
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfoBuilder::new()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    // only one viewport but vec! cause it is possible to have many
    let viewports = vec![vk::ViewportBuilder::new()
        .x(0.0)
        .y(0.0)
        .width(surface_capabilities.current_extent.width as f32)
        .height(surface_capabilities.current_extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0)];

    // similar story to viewports and fills the whole screen
    let scissors = vec![vk::Rect2DBuilder::new()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(surface_capabilities.current_extent)];

    // pretty normal setttings, backface culling, clockwise front facing
    let rasterizer = vk::PipelineRasterizationStateCreateInfoBuilder::new()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::CLOCKWISE);

    // disabled for now
    let multisampling = vk::PipelineMultisampleStateCreateInfoBuilder::new()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlagBits::_1);

    // alpha blending (src is new color) i.e.
    // finalColor.rgb = newAlpha * newColor + (1 - newAlpha) * oldColor;
    // finalColor.a = newAlpha.a;
    let color_blend_attachments = vec![vk::PipelineColorBlendAttachmentStateBuilder::new()
        .color_write_mask(
            vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
        )
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .color_blend_op(vk::BlendOp::ADD)
        .alpha_blend_op(vk::BlendOp::ADD)];

    let pipeline_layout_info = vk::PipelineLayoutCreateInfoBuilder::new();
    let pipeline_layout =
        unsafe { device.create_pipeline_layout(&pipeline_layout_info, None, None) }.unwrap();

    (
        vertex_input,
        input_assembly,
        viewports,
        scissors,
        rasterizer,
        multisampling,
        color_blend_attachments,
        pipeline_layout,
    )
}

fn create_render_pass(format: vk::SurfaceFormatKHR, device: &DeviceLoader) -> vk::RenderPass {
    // clear framebuffer before render & optimize final_layout for presentation in swapchain
    let attachments = vec![vk::AttachmentDescriptionBuilder::new()
        .format(format.format)
        .samples(vk::SampleCountFlagBits::_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)];

    // one only attachment & subpass used
    let color_attachment_references = vec![vk::AttachmentReferenceBuilder::new()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];

    let subpasses =
        vec![vk::SubpassDescriptionBuilder::new().color_attachments(&color_attachment_references)];

    let render_pass_info = vk::RenderPassCreateInfoBuilder::new()
        .attachments(&attachments)
        .subpasses(&subpasses);
    let render_pass = unsafe { device.create_render_pass(&render_pass_info, None, None) }.unwrap();

    render_pass
}

pub fn create_graphics_pipeline<'a>(
    device: &DeviceLoader,
    shader_vert: vk::ShaderModule,
    shader_frag: vk::ShaderModule,
    surface_capabilities: SurfaceCapabilitiesKHR,

    format: vk::SurfaceFormatKHR,
) -> (vk::Pipeline, vk::PipelineLayout, vk::RenderPass) {
    // create fixed functions
    let (
        vertex_input,
        input_assembly,
        viewports,
        scissors,
        rasterizer,
        multisampling,
        color_blend_attachments,
        pipeline_layout,
    ) = create_fixed_functions(surface_capabilities, device);

    // make the borrow checker happy and create it here :)
    let viewport_state = vk::PipelineViewportStateCreateInfoBuilder::new()
        .viewports(&viewports)
        .scissors(&scissors);

    // make the borrow checker happy and create it here :)
    let color_blending = vk::PipelineColorBlendStateCreateInfoBuilder::new()
        .logic_op_enable(false)
        .attachments(&color_blend_attachments);

    // create shader stages
    let shader_stages = vec![
        vk::PipelineShaderStageCreateInfoBuilder::new()
            .stage(vk::ShaderStageFlagBits::VERTEX)
            .module(shader_vert)
            .name(unsafe { CStr::from_ptr(SHADER_ENTRY) }),
        vk::PipelineShaderStageCreateInfoBuilder::new()
            .stage(vk::ShaderStageFlagBits::FRAGMENT)
            .module(shader_frag)
            .name(unsafe { CStr::from_ptr(SHADER_ENTRY) }),
    ];

    // create render_pass
    let render_pass = create_render_pass(format, device);

    // le big info struct
    let pipeline_info = vk::GraphicsPipelineCreateInfoBuilder::new()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterizer)
        .multisample_state(&multisampling)
        .color_blend_state(&color_blending)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0);

    // graphics pipeline
    let pipeline =
        unsafe { device.create_graphics_pipelines(None, &[pipeline_info], None) }.unwrap()[0];

    (pipeline, pipeline_layout, render_pass)
}
