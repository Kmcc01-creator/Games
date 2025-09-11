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
        "rgba8_unorm" | "r8g8b8a8_unorm" | "u8x4_norm" => vk::Format::R8G8B8A8_UNORM,
        "rgba8_srgb" | "r8g8b8a8_srgb" => vk::Format::R8G8B8A8_SRGB,
        "bgra8_unorm" | "b8g8r8a8_unorm" => vk::Format::B8G8R8A8_UNORM,
        "bgra8_srgb" | "b8g8r8a8_srgb" => vk::Format::B8G8R8A8_SRGB,
        "rgba16_unorm" | "r16g16b16a16_unorm" => vk::Format::R16G16B16A16_UNORM,
        "rgba16f" | "r16g16b16a16_sfloat" => vk::Format::R16G16B16A16_SFLOAT,
        "r16f" | "r16_sfloat" => vk::Format::R16_SFLOAT,
        "rg16f" | "r16g16_sfloat" => vk::Format::R16G16_SFLOAT,
        "r32f" | "r32_sfloat" => vk::Format::R32_SFLOAT,
        "rg32f" | "r32g32_sfloat" => vk::Format::R32G32_SFLOAT,
        "rgb32f" | "r32g32b32_sfloat" => vk::Format::R32G32B32_SFLOAT,
        "rgba32f" | "r32g32b32a32_sfloat" => vk::Format::R32G32B32A32_SFLOAT,
        "rgb10a2_unorm" | "a2b10g10r10_unorm" => vk::Format::A2B10G10R10_UNORM_PACK32,
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

pub fn color_blend_attachments_from(desc: &PipelineDesc) -> Vec<vk::PipelineColorBlendAttachmentState> {
    if let Some(colors) = desc.color_targets {
        if !colors.is_empty() {
            return colors
                .iter()
                .map(|c| {
                    let enable = c.blend.unwrap_or_else(|| desc.blend.as_ref().map(|b| b.enable).unwrap_or(false));
                    vk::PipelineColorBlendAttachmentState::builder()
                        .color_write_mask(
                            vk::ColorComponentFlags::R
                                | vk::ColorComponentFlags::G
                                | vk::ColorComponentFlags::B
                                | vk::ColorComponentFlags::A,
                        )
                        .blend_enable(enable)
                        .build()
                })
                .collect();
        }
    }
    vec![color_blend_attachment_from(desc)]
}

// Public helpers to map color/depth format strings to Vulkan formats
pub fn parse_color_format(s: &str) -> Option<vk::Format> {
    let f = match s.to_ascii_lowercase().as_str() {
        // 8-bit
        "rgba8_unorm" | "r8g8b8a8_unorm" => vk::Format::R8G8B8A8_UNORM,
        "rgba8_srgb" | "r8g8b8a8_srgb" => vk::Format::R8G8B8A8_SRGB,
        "bgra8_unorm" | "b8g8r8a8_unorm" => vk::Format::B8G8R8A8_UNORM,
        "bgra8_srgb" | "b8g8r8a8_srgb" => vk::Format::B8G8R8A8_SRGB,
        // 10-bit
        "rgb10a2_unorm" | "a2b10g10r10_unorm" => vk::Format::A2B10G10R10_UNORM_PACK32,
        // 16-bit
        "rgba16_unorm" | "r16g16b16a16_unorm" => vk::Format::R16G16B16A16_UNORM,
        "rgba16f" | "r16g16b16a16_sfloat" => vk::Format::R16G16B16A16_SFLOAT,
        "rg16f" | "r16g16_sfloat" => vk::Format::R16G16_SFLOAT,
        "r16f" | "r16_sfloat" => vk::Format::R16_SFLOAT,
        // 32-bit float
        "r32f" | "r32_sfloat" => vk::Format::R32_SFLOAT,
        "rg32f" | "r32g32_sfloat" => vk::Format::R32G32_SFLOAT,
        "rgb32f" | "r32g32b32_sfloat" => vk::Format::R32G32B32_SFLOAT,
        "rgba32f" | "r32g32b32a32_sfloat" => vk::Format::R32G32B32A32_SFLOAT,
        _ => return None,
    };
    Some(f)
}

pub fn parse_depth_format(s: &str) -> Option<vk::Format> {
    match s {
        "D32_SFLOAT" | "d32_sfloat" => Some(vk::Format::D32_SFLOAT),
        "D24_UNORM_S8_UINT" | "d24_unorm_s8_uint" => Some(vk::Format::D24_UNORM_S8_UINT),
        "D32_SFLOAT_S8_UINT" | "d32_sfloat_s8_uint" => Some(vk::Format::D32_SFLOAT_S8_UINT),
        "D16_UNORM" | "d16_unorm" => Some(vk::Format::D16_UNORM),
        _ => None,
    }
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
