use std::os::raw::c_char;

use erupt::{cstr, utils, vk, DeviceLoader};

use crate::application::model;

use std::ffi::CStr;

// shader spvs
pub const SHADER_VERT: &[u8] = include_bytes!("../../res/shaders/vert.spv");
pub const SHADER_FRAG: &[u8] = include_bytes!("../../res/shaders/frag.spv");

pub const SHADER_ENTRY: *const c_char = cstr!("main");

pub fn create_shader_modules<'a>(device: &DeviceLoader) -> (vk::ShaderModule, vk::ShaderModule) {
    // vertex shader
    let vert_decoded = utils::decode_spv(SHADER_VERT).expect("Failed to decode vertex shader spv");
    let module_info = vk::ShaderModuleCreateInfoBuilder::new().code(&vert_decoded);
    let shader_vert = unsafe { device.create_shader_module(&module_info, None, None) }
        .expect("Failed to create vertex shader module!");

    // fragment shader
    let frag_decoded =
        utils::decode_spv(SHADER_FRAG).expect("Failed to decode fragment shader spv");
    let module_info = vk::ShaderModuleCreateInfoBuilder::new().code(&frag_decoded);
    let shader_frag = unsafe { device.create_shader_module(&module_info, None, None) }
        .expect("Failed to create fragment shader module!");

    (shader_vert, shader_frag)
}

// stuff for graphics pipeline bar render pass, pipeline layout (WIP) and shader stages
fn create_fixed_functions<'a>(
    device: &'a DeviceLoader,
    descriptor_set_layouts: &[vk::DescriptorSetLayout],
) -> (
    vk::PipelineInputAssemblyStateCreateInfoBuilder<'a>,
    vk::PipelineRasterizationStateCreateInfoBuilder<'a>,
    vk::PipelineMultisampleStateCreateInfoBuilder<'a>,
    Vec<vk::PipelineColorBlendAttachmentStateBuilder<'a>>,
    vk::PipelineLayout,
) {
    // how vertices are interpreted, TRIANGLE_LIST is just regular triangles not triangle strip
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfoBuilder::new()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    // pretty normal setttings, no backface culling, clockwise front facing
    let rasterizer = vk::PipelineRasterizationStateCreateInfoBuilder::new()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE);

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

    let pipeline_layout_info =
        vk::PipelineLayoutCreateInfoBuilder::new().set_layouts(descriptor_set_layouts);
    let pipeline_layout =
        unsafe { device.create_pipeline_layout(&pipeline_layout_info, None, None) }
            .expect("Failed to create pipeline layout!");

    (
        input_assembly,
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

    // subpass dependency to trigger render_finished_semaphore
    let dependencies = vec![vk::SubpassDependencyBuilder::new()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)];

    let render_pass_info = vk::RenderPassCreateInfoBuilder::new()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    let render_pass = unsafe { device.create_render_pass(&render_pass_info, None, None) }
        .expect("Failed to expect render pass!");

    render_pass
}

pub fn create_descriptor_pool(device: &DeviceLoader, swapchain_length: u32) -> vk::DescriptorPool {
    let pool_size = &[vk::DescriptorPoolSizeBuilder::new()
        ._type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(swapchain_length)];

    let pool_info = vk::DescriptorPoolCreateInfoBuilder::new()
        .pool_sizes(pool_size)
        .max_sets(swapchain_length);

    unsafe { device.create_descriptor_pool(&pool_info, None, None) }
        .expect("Failed to create descriptor set!")
}

pub fn create_descriptor_set_layout(device: &DeviceLoader) -> vk::DescriptorSetLayout {
    let binding = &[vk::DescriptorSetLayoutBindingBuilder::new()
        .binding(0)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX)];

    let create_info = vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(binding);

    unsafe { device.create_descriptor_set_layout(&create_info, None, None) }
        .expect("Failed to create descriptor set layout!")
}

pub fn create_descriptor_sets(
    device: &DeviceLoader,
    layout: &vk::DescriptorSetLayout,
    pool: &vk::DescriptorPool,
    uniform_buffer: &Vec<vk::Buffer>,
    swapchain_length: usize,
) -> Vec<vk::DescriptorSet> {
    let layouts: &Vec<vk::DescriptorSetLayout> = &vec![*layout; swapchain_length];

    let alloc_info = vk::DescriptorSetAllocateInfoBuilder::new()
        .set_layouts(layouts)
        .descriptor_pool(*pool);

    let descriptor_sets = unsafe { device.allocate_descriptor_sets(&alloc_info) }
        .expect("Failed to allocate descriptor sets!");

    for (index, set) in descriptor_sets.iter().enumerate() {
        let buffer_info = &[vk::DescriptorBufferInfoBuilder::new()
            .buffer(uniform_buffer[index])
            .range(vk::WHOLE_SIZE)];
        let descriptor_write = &[vk::WriteDescriptorSetBuilder::new()
            .dst_set(*set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(buffer_info)];

        unsafe {
            device.update_descriptor_sets(descriptor_write, &[]);
        }
    }
    descriptor_sets
}

pub fn create_graphics_pipeline<'a>(
    device: &DeviceLoader,
    shader_vert: vk::ShaderModule,
    shader_frag: vk::ShaderModule,
    descriptor_set_layout: &vk::DescriptorSetLayout,
    format: vk::SurfaceFormatKHR,
) -> (vk::Pipeline, vk::PipelineLayout, vk::RenderPass) {
    // vertex info
    let binding_descriptions = [model::Vertex::get_binding_descriptions()];
    let attribute_descriptions = model::Vertex::get_attribute_descriptions();

    let vertex_input = vk::PipelineVertexInputStateCreateInfoBuilder::new()
        .vertex_binding_descriptions(&binding_descriptions)
        .vertex_attribute_descriptions(&attribute_descriptions);

    // create fixed functions
    let (input_assembly, rasterizer, multisampling, color_blend_attachments, pipeline_layout) =
        create_fixed_functions(device, &[*descriptor_set_layout]);

    // make the borrow checker happy and create it here :)
    let viewport_state = vk::PipelineViewportStateCreateInfoBuilder::new()
        .viewport_count(1)
        .scissor_count(1);

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

    // dynamic states (for resizing)
    let dynamic_states = vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

    let dynamic_states_info =
        vk::PipelineDynamicStateCreateInfoBuilder::new().dynamic_states(&dynamic_states);

    // le big info struct
    let pipeline_info = vk::GraphicsPipelineCreateInfoBuilder::new()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterizer)
        .multisample_state(&multisampling)
        .color_blend_state(&color_blending)
        .dynamic_state(&dynamic_states_info)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0);

    // graphics pipeline
    let pipeline = unsafe { device.create_graphics_pipelines(None, &[pipeline_info], None) }
        .expect("Failed to create graphics pipeline!")[0];

    (pipeline, pipeline_layout, render_pass)
}
