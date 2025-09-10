#![cfg(feature = "vulkan-linux")]
use ash::vk;
use crate::resources::{ResourceBindings, BindingStages, VertexLayout, StepMode};
use crate::pipeline::{PipelineDesc, RasterState as Rs, PolygonMode as Pm, CullMode as Cm, FrontFace as Ff, CompareOp, PushConstantRange, StageMask};
use std::collections::BTreeMap;

pub fn stage_flags_from_binding_stages(st: &Option<BindingStages>) -> vk::ShaderStageFlags {
    if let Some(s) = st {
        let mut f = vk::ShaderStageFlags::empty();
        if s.vs { f |= vk::ShaderStageFlags::VERTEX; }
        if s.fs { f |= vk::ShaderStageFlags::FRAGMENT; }
        if s.cs { f |= vk::ShaderStageFlags::COMPUTE; }
        if f.is_empty() { vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT } else { f }
    } else {
        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT
    }
}

pub fn descriptor_bindings_from<RB: ResourceBindings>() -> BTreeMap<u32, Vec<vk::DescriptorSetLayoutBinding>> {
    use crate::resources::ResourceKind;
    let mut by_set: BTreeMap<u32, Vec<vk::DescriptorSetLayoutBinding>> = BTreeMap::new();
    for b in RB::bindings() {
        let dtype = match b.kind {
            ResourceKind::Uniform => vk::DescriptorType::UNIFORM_BUFFER,
            ResourceKind::Texture => vk::DescriptorType::SAMPLED_IMAGE,
            ResourceKind::Sampler => vk::DescriptorType::SAMPLER,
            ResourceKind::CombinedImageSampler => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        };
        let stage_flags = stage_flags_from_binding_stages(&b.stages);
        let bind = vk::DescriptorSetLayoutBinding::builder()
            .binding(b.binding)
            .descriptor_type(dtype)
            .descriptor_count(1)
            .stage_flags(stage_flags)
            .build();
        by_set.entry(b.set).or_default().push(bind);
    }
    for v in by_set.values_mut() { v.sort_by_key(|b| b.binding); }
    by_set
}

fn map_format(fmt: &str) -> vk::Format {
    match fmt {
        "f32" => vk::Format::R32_SFLOAT,
        "i32" => vk::Format::R32_SINT,
        "u32" => vk::Format::R32_UINT,
        "vec2" => vk::Format::R32G32_SFLOAT,
        "vec3" => vk::Format::R32G32B32_SFLOAT,
        "vec4" => vk::Format::R32G32B32A32_SFLOAT,
        "rgba8_unorm" | "u8x4_norm" => vk::Format::R8G8B8A8_UNORM,
        _ => vk::Format::R32G32B32A32_SFLOAT,
    }
}

pub fn vertex_input_from<VL: VertexLayout>() -> (Vec<vk::VertexInputBindingDescription>, Vec<vk::VertexInputAttributeDescription>) {
    let mut binding_descs: Vec<vk::VertexInputBindingDescription> = Vec::new();
    for vb in VL::vertex_buffers() {
        let input_rate = match vb.step { StepMode::Vertex => vk::VertexInputRate::VERTEX, StepMode::Instance => vk::VertexInputRate::INSTANCE };
        binding_descs.push(vk::VertexInputBindingDescription { binding: vb.binding, stride: vb.stride, input_rate });
    }
    let mut attr_descs: Vec<vk::VertexInputAttributeDescription> = Vec::new();
    for a in VL::vertex_attrs() {
        attr_descs.push(vk::VertexInputAttributeDescription { location: a.location, binding: a.binding, format: map_format(a.format), offset: a.offset });
    }
    (binding_descs, attr_descs)
}

pub fn raster_state_from(desc: &PipelineDesc) -> (vk::PolygonMode, vk::CullModeFlags, vk::FrontFace) {
    let rs = desc.raster.clone().unwrap_or(Rs { polygon: Pm::Fill, cull: Cm::Back, front_face: Ff::Cw });
    let poly = match rs.polygon { Pm::Fill => vk::PolygonMode::FILL, Pm::Line => vk::PolygonMode::LINE };
    let cull = match rs.cull { Cm::None => vk::CullModeFlags::NONE, Cm::Front => vk::CullModeFlags::FRONT, Cm::Back => vk::CullModeFlags::BACK };
    let ff = match rs.front_face { Ff::Cw => vk::FrontFace::CLOCKWISE, Ff::Ccw => vk::FrontFace::COUNTER_CLOCKWISE };
    (poly, cull, ff)
}

pub fn samples_from(desc: &PipelineDesc) -> vk::SampleCountFlags {
    match desc.samples.unwrap_or(1) { 1 => vk::SampleCountFlags::TYPE_1, 2 => vk::SampleCountFlags::TYPE_2, 4 => vk::SampleCountFlags::TYPE_4, 8 => vk::SampleCountFlags::TYPE_8, _ => vk::SampleCountFlags::TYPE_1 }
}

pub fn color_blend_attachment_from(desc: &PipelineDesc) -> vk::PipelineColorBlendAttachmentState {
    let enable = desc.blend.as_ref().map(|b| b.enable).unwrap_or(false);
    vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A)
        .blend_enable(enable)
        .build()
}

pub fn depth_stencil_from(desc: &PipelineDesc) -> vk::PipelineDepthStencilStateCreateInfo {
    if let Some(ds) = &desc.depth_stencil {
        let compare = match ds.compare {
            CompareOp::Never => vk::CompareOp::NEVER,
            CompareOp::Less => vk::CompareOp::LESS,
            CompareOp::Equal => vk::CompareOp::EQUAL,
            CompareOp::LessOrEqual => vk::CompareOp::LESS_OR_EQUAL,
            CompareOp::Greater => vk::CompareOp::GREATER,
            CompareOp::NotEqual => vk::CompareOp::NOT_EQUAL,
            CompareOp::GreaterOrEqual => vk::CompareOp::GREATER_OR_EQUAL,
            CompareOp::Always => vk::CompareOp::ALWAYS,
        };
        vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(ds.test)
            .depth_write_enable(ds.write)
            .depth_compare_op(compare)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false)
            .build()
    } else {
        vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false)
            .build()
    }
}

pub fn push_constant_ranges_from(desc: &PipelineDesc) -> Vec<vk::PushConstantRange> {
    if let Some(pc) = &desc.push_constants {
        let mut flags = vk::ShaderStageFlags::empty();
        if let Some(StageMask { vs, fs, cs }) = pc.stages.clone() { if vs { flags |= vk::ShaderStageFlags::VERTEX; } if fs { flags |= vk::ShaderStageFlags::FRAGMENT; } if cs { flags |= vk::ShaderStageFlags::COMPUTE; } }
        if flags.is_empty() { flags = vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT; }
        vec![vk::PushConstantRange { stage_flags: flags, offset: 0, size: pc.size }]
    } else { Vec::new() }
}

pub fn dynamic_states_from(desc: &PipelineDesc) -> Vec<vk::DynamicState> {
    if let Some(d) = &desc.dynamic {
        let mut v = Vec::new();
        if d.viewport { v.push(vk::DynamicState::VIEWPORT); }
        if d.scissor { v.push(vk::DynamicState::SCISSOR); }
        v
    } else { Vec::new() }
}

