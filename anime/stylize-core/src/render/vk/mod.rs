//! Minimal Vulkan setup using Ash (feature-gated).
//! Creates instance, picks a physical device, and creates a logical device
//! with a graphics queue. This is an initial scaffold for NPR passes.

use anyhow::{anyhow, Result};
use std::ffi::CString;

pub struct VkContext {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub pdevice: ash::vk::PhysicalDevice,
    pub device: ash::Device,
    pub graphics_queue: ash::vk::Queue,
    pub graphics_queue_family: u32,
}

impl VkContext {
    pub fn new(app_name: &str) -> Result<Self> {
        let entry = unsafe { ash::Entry::load()? };

        let app_name_c = CString::new(app_name).unwrap();
        let engine_name_c = CString::new("stylize-core").unwrap();
        let app_info = ash::vk::ApplicationInfo::builder()
            .application_name(&app_name_c)
            .application_version(ash::vk::make_api_version(0, 0, 1, 0))
            .engine_name(&engine_name_c)
            .engine_version(ash::vk::make_api_version(0, 0, 1, 0))
            .api_version(ash::vk::API_VERSION_1_3);

        let instance_ci = ash::vk::InstanceCreateInfo::builder().application_info(&app_info);
        let instance = unsafe { entry.create_instance(&instance_ci, None)? };

        // Pick a physical device with graphics queue
        let pdevices = unsafe { instance.enumerate_physical_devices()? };
        let (pdevice, graphics_queue_family) = pdevices
            .iter()
            .find_map(|pd| {
                let families = unsafe { instance.get_physical_device_queue_family_properties(*pd) };
                families
                    .iter()
                    .enumerate()
                    .find(|(_, f)| f.queue_flags.contains(ash::vk::QueueFlags::GRAPHICS))
                    .map(|(idx, _)| (*pd, idx as u32))
            })
            .ok_or_else(|| anyhow!("No suitable physical device with graphics queue"))?;

        let priorities = [1.0f32];
        let queue_ci = [ash::vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_family)
            .queue_priorities(&priorities)
            .build()];

    // Enable dynamic rendering (Vulkan 1.3)
    let mut v13 = ash::vk::PhysicalDeviceVulkan13Features::builder().dynamic_rendering(true);
    let device_ci = ash::vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_ci)
        .push_next(&mut v13);
        let device = unsafe { instance.create_device(pdevice, &device_ci, None)? };
        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_family, 0) };

        Ok(Self { entry, instance, pdevice, device, graphics_queue, graphics_queue_family })
    }

    /// Create a minimal placeholder pipeline layout for future NPR passes.
    pub fn make_pipeline_layout(&self) -> Result<ash::vk::PipelineLayout> {
        let layout_ci = ash::vk::PipelineLayoutCreateInfo::builder();
        let layout = unsafe { self.device.create_pipeline_layout(&layout_ci, None)? };
        Ok(layout)
    }

    pub fn device_name(&self) -> String {
        let props = unsafe { self.instance.get_physical_device_properties(self.pdevice) };
        // Convert C string to Rust string, trimming trailing nulls
        let raw = unsafe { std::ffi::CStr::from_ptr(props.device_name.as_ptr()) };
        raw.to_string_lossy().into_owned()
    }
}

impl Drop for VkContext {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().ok();
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

#[repr(C)]
pub struct ToonStyle {
    pub row0: [[f32; 4]; 8], // shadow, mid, rimStrength, rimWidth
    pub row1: [[f32; 4]; 8], // softness, hueShad, hueLit, satShad
    pub row2: [[f32; 4]; 8], // satLit, specThr, specInt, pad
}

impl Default for ToonStyle {
    fn default() -> Self {
        let mut s = Self {
            row0: [[0.60, -1.0, 0.20, 0.35]; 8],
            row1: [[0.05, -6.0, 6.0, 0.95]; 8],
            row2: [[1.05, 0.86, 0.22, 0.0]; 8],
        };
        // Slight difference for skin (id 0)
        s.row0[0][0] = 0.63; // face threshold
        s
    }
}

pub fn toon_style_from_dna(d: &crate::asset_dna::schema::Shading) -> ToonStyle {
    let mut s = ToonStyle::default();
    // IDs: 0=skin, 1=hair, 2=cloth (others copy cloth)
    s.row0[0][0] = d.face_shadow_threshold;
    s.row0[1][0] = d.cloth_shadow_threshold; // hair uses cloth for now
    s.row0[2][0] = d.cloth_shadow_threshold;
    for id in 0..8 {
        s.row0[id][1] = if d.bands >= 3 { 0.75 } else { -1.0 }; // mid band threshold heuristic
        s.row0[id][2] = d.rim_strength;
        s.row0[id][3] = d.rim_width;

        s.row1[id][0] = d.band_softness;
        s.row1[id][1] = d.hue_shift_shadow_deg;
        s.row1[id][2] = d.hue_shift_light_deg;
        s.row1[id][3] = d.sat_scale_shadow;

        s.row2[id][0] = d.sat_scale_light;
        s.row2[id][1] = d.spec_threshold;
        s.row2[id][2] = d.spec_intensity;
        s.row2[id][3] = 0.0;
    }
    s
}

pub fn enumerate_devices() -> Result<Vec<String>> {
    let entry = unsafe { ash::Entry::load()? };
    let app_name_c = CString::new("stylize-enum").unwrap();
    let engine_name_c = CString::new("stylize-core").unwrap();
    let app_info = ash::vk::ApplicationInfo::builder()
        .application_name(&app_name_c)
        .application_version(ash::vk::make_api_version(0, 0, 1, 0))
        .engine_name(&engine_name_c)
        .engine_version(ash::vk::make_api_version(0, 0, 1, 0))
        .api_version(ash::vk::API_VERSION_1_3);
    let instance_ci = ash::vk::InstanceCreateInfo::builder().application_info(&app_info);
    let instance = unsafe { entry.create_instance(&instance_ci, None)? };
    let mut out = Vec::new();
    for pd in unsafe { instance.enumerate_physical_devices()? } {
        let props = unsafe { instance.get_physical_device_properties(pd) };
        let name = unsafe { std::ffi::CStr::from_ptr(props.device_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        out.push(format!("{} (API {}.{}.{})", name, ash::vk::api_version_major(props.api_version), ash::vk::api_version_minor(props.api_version), ash::vk::api_version_patch(props.api_version)));
    }
    unsafe { instance.destroy_instance(None) };
    Ok(out)
}

/// Placeholder for dynamic rendering setup for G-buffer.
pub fn describe_dynamic_rendering() -> &'static str {
    "Vulkan dynamic rendering: G-buffer → toon → outline"
}

/// Stub types for NPR pipeline stages under Vulkan.
pub struct GBufferPass;
pub struct ToonPass;
pub struct OutlinePass;

impl GBufferPass {
    pub fn new(_ctx: &VkContext) -> Result<Self> { Ok(Self) }
}
impl ToonPass {
    pub fn new(_ctx: &VkContext) -> Result<Self> { Ok(Self) }
}
impl OutlinePass {
    pub fn new(_ctx: &VkContext) -> Result<Self> { Ok(Self) }
}

pub struct NprPipeline {
    pub gbuffer: GBufferPass,
    pub toon: ToonPass,
    pub outline: OutlinePass,
}

impl NprPipeline {
    pub fn new(ctx: &VkContext) -> Result<Self> {
        Ok(Self {
            gbuffer: GBufferPass::new(ctx)?,
            toon: ToonPass::new(ctx)?,
            outline: OutlinePass::new(ctx)?,
        })
    }
}

// Embedded SPIR-V compiled at build time by build.rs
// Paths are under OUT_DIR and named after the source filename with `.spv` appended.
pub const TOON_FRAG_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/toon.frag.spv"));
pub const TOON_VERT_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/toon.vert.spv"));
pub const OUTLINE_VERT_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/outline.vert.spv"));
pub const OUTLINE_FRAG_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/outline.frag.spv"));
pub const GBUFFER_VERT_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/gbuffer.vert.spv"));
pub const GBUFFER_FRAG_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/gbuffer.frag.spv"));
pub const FSQ_VERT_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/fsq.vert.spv"));
pub const TOON_GBUFFER_FRAG_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/toon_gbuffer.frag.spv"));
pub const MESH_GBUFFER_VERT_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/mesh_gbuffer.vert.spv"));
pub const MESH_GBUFFER_FRAG_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/mesh_gbuffer.frag.spv"));

pub fn create_shader_module(device: &ash::Device, bytes: &[u8]) -> Result<ash::vk::ShaderModule> {
    use ash::{util, vk};
    let mut cursor = std::io::Cursor::new(bytes);
    let code = util::read_spv(&mut cursor)?;
    let info = vk::ShaderModuleCreateInfo::builder().code(&code);
    let module = unsafe { device.create_shader_module(&info, None)? };
    Ok(module)
}

fn find_memory_type(instance: &ash::Instance, pdevice: ash::vk::PhysicalDevice, type_bits: u32, props: ash::vk::MemoryPropertyFlags) -> Result<u32> {
    let mem_props = unsafe { instance.get_physical_device_memory_properties(pdevice) };
    for i in 0..mem_props.memory_type_count {
        let i = i as usize;
        if (type_bits & (1 << i)) != 0 && mem_props.memory_types[i].property_flags.contains(props) {
            return Ok(i as u32);
        }
    }
    Err(anyhow!("No suitable memory type"))
}

pub fn render_offscreen_rgba(ctx: &VkContext, width: u32, height: u32) -> Result<Vec<u8>> {
    use ash::vk as vk;

    // Create offscreen color image
    let format = vk::Format::R8G8B8A8_UNORM;
    let image_ci = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .format(format)
        .extent(vk::Extent3D { width, height, depth: 1 })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC)
        .initial_layout(vk::ImageLayout::UNDEFINED);
    let image = unsafe { ctx.device.create_image(&image_ci, None)? };
    let mem_reqs = unsafe { ctx.device.get_image_memory_requirements(image) };
    let mem_type = find_memory_type(&ctx.instance, ctx.pdevice, mem_reqs.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL)?;
    let alloc = vk::MemoryAllocateInfo::builder().allocation_size(mem_reqs.size).memory_type_index(mem_type);
    let image_mem = unsafe { ctx.device.allocate_memory(&alloc, None)? };
    unsafe { ctx.device.bind_image_memory(image, image_mem, 0)? };

    let view_ci = vk::ImageViewCreateInfo::builder()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });
    let image_view = unsafe { ctx.device.create_image_view(&view_ci, None)? };

    // Create staging buffer to copy image to host
    let buffer_size = (width as usize * height as usize * 4) as u64;
    let buf_ci = vk::BufferCreateInfo::builder()
        .size(buffer_size)
        .usage(vk::BufferUsageFlags::TRANSFER_DST)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = unsafe { ctx.device.create_buffer(&buf_ci, None)? };
    let buf_reqs = unsafe { ctx.device.get_buffer_memory_requirements(buffer) };
    let host_mem_type = find_memory_type(
        &ctx.instance,
        ctx.pdevice,
        buf_reqs.memory_type_bits,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    let buf_alloc = vk::MemoryAllocateInfo::builder().allocation_size(buf_reqs.size).memory_type_index(host_mem_type);
    let buffer_mem = unsafe { ctx.device.allocate_memory(&buf_alloc, None)? };
    unsafe { ctx.device.bind_buffer_memory(buffer, buffer_mem, 0)? };

    // Command pool + command buffer
    let pool_ci = vk::CommandPoolCreateInfo::builder().queue_family_index(ctx.graphics_queue_family);
    let cmd_pool = unsafe { ctx.device.create_command_pool(&pool_ci, None)? };
    let alloc_ci = vk::CommandBufferAllocateInfo::builder().command_pool(cmd_pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1);
    let cmd_buf = unsafe { ctx.device.allocate_command_buffers(&alloc_ci)? }[0];

    // Create pipeline for full-screen triangle with toon shading
    let vert = create_shader_module(&ctx.device, TOON_VERT_SPV)?;
    let frag = create_shader_module(&ctx.device, TOON_FRAG_SPV)?;
    let stage_infos = [
        vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::VERTEX).module(vert).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
        vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::FRAGMENT).module(frag).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
    ];

    // Push constants for toon params (12 floats = 48 bytes)
    let pc_range = vk::PushConstantRange::builder()
        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
        .offset(0)
        .size(48)
        .build();
    let layout = vk::PipelineLayoutCreateInfo::builder().push_constant_ranges(std::slice::from_ref(&pc_range));
    let pipeline_layout = unsafe { ctx.device.create_pipeline_layout(&layout, None)? };

    let ia = vk::PipelineInputAssemblyStateCreateInfo::builder().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    let vp = vk::PipelineViewportStateCreateInfo::builder().viewport_count(1).scissor_count(1);
    let rs = vk::PipelineRasterizationStateCreateInfo::builder().polygon_mode(vk::PolygonMode::FILL).cull_mode(vk::CullModeFlags::NONE).front_face(vk::FrontFace::COUNTER_CLOCKWISE).line_width(1.0);
    let ms = vk::PipelineMultisampleStateCreateInfo::builder().rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let cb_mask = vk::ColorComponentFlags::R
        | vk::ColorComponentFlags::G
        | vk::ColorComponentFlags::B
        | vk::ColorComponentFlags::A;
    let cb = vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(cb_mask)
        .blend_enable(false)
        .build();
    let cb_state = vk::PipelineColorBlendStateCreateInfo::builder().attachments(std::slice::from_ref(&cb));
    let ds = vk::PipelineDepthStencilStateCreateInfo::builder().depth_test_enable(false).depth_write_enable(false);

    // Dynamic states for viewport/scissor
    let dyn_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dyn_state = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dyn_states);

    // Dynamic rendering pipeline info
    let mut rendering_info = vk::PipelineRenderingCreateInfo::builder()
        .color_attachment_formats(std::slice::from_ref(&format));

    let vi = vk::PipelineVertexInputStateCreateInfo::default();
    let vpci = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&stage_infos)
        .vertex_input_state(&vi)
        .input_assembly_state(&ia)
        .viewport_state(&vp)
        .rasterization_state(&rs)
        .multisample_state(&ms)
        .depth_stencil_state(&ds)
        .color_blend_state(&cb_state)
        .dynamic_state(&dyn_state)
        .layout(pipeline_layout)
        .push_next(&mut rendering_info);

    let pipeline = unsafe { ctx.device.create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&vpci), None) }
        .map_err(|e| anyhow!("pipeline creation failed: {:?}", e.1))?
        [0];

    // Record commands
    let begin = vk::CommandBufferBeginInfo::builder();
    unsafe { ctx.device.begin_command_buffer(cmd_buf, &begin)? };

    // Transition image to COLOR_ATTACHMENT_OPTIMAL
    let barrier_to_color = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
    unsafe {
        ctx.device.cmd_pipeline_barrier(
            cmd_buf,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            std::slice::from_ref(&barrier_to_color),
        );
    }

    // Begin dynamic rendering
    let clear = vk::ClearValue { color: vk::ClearColorValue { float32: [0.04, 0.04, 0.06, 1.0] } };
    let color_attachment = vk::RenderingAttachmentInfo::builder()
        .image_view(image_view)
        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .clear_value(clear);
    let render_info = vk::RenderingInfo::builder()
        .render_area(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } })
        .layer_count(1)
        .color_attachments(std::slice::from_ref(&color_attachment));
    unsafe {
        ctx.device.cmd_begin_rendering(cmd_buf, &render_info);
        // Set viewport/scissor
        let viewport = vk::Viewport { x: 0.0, y: 0.0, width: width as f32, height: height as f32, min_depth: 0.0, max_depth: 1.0 };
        let scissor = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } };
        ctx.device.cmd_set_viewport(cmd_buf, 0, std::slice::from_ref(&viewport));
        ctx.device.cmd_set_scissor(cmd_buf, 0, std::slice::from_ref(&scissor));
        // Bind pipeline and push toon params
        ctx.device.cmd_bind_pipeline(cmd_buf, vk::PipelineBindPoint::GRAPHICS, pipeline);
        #[repr(C)]
        struct ToonPC { data: [f32; 12] }
        let pc = ToonPC { data: [
            0.60,   // shadowThreshold
            -1.0,   // midThreshold (disabled)
            0.25,   // rimStrength
            0.35,   // rimWidth
            0.05,   // bandSoftness
            0.0,    // hueShiftShadowDeg
            0.0,    // hueShiftLightDeg
            1.00,   // satScaleShadow
            1.00,   // satScaleLight
            0.86,   // specThreshold
            0.25,   // specIntensity
            0.0,    // _pad
        ]};
        let bytes = std::slice::from_raw_parts((&pc as *const ToonPC) as *const u8, std::mem::size_of::<ToonPC>());
        ctx.device.cmd_push_constants(
            cmd_buf,
            pipeline_layout,
            vk::ShaderStageFlags::FRAGMENT,
            0,
            bytes,
        );
        // Draw fullscreen triangle
        ctx.device.cmd_draw(cmd_buf, 3, 1, 0, 0);
        ctx.device.cmd_end_rendering(cmd_buf);
    }

    // Transition image to TRANSFER_SRC_OPTIMAL for copy
    let barrier_to_src = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
    unsafe {
        ctx.device.cmd_pipeline_barrier(
            cmd_buf,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            std::slice::from_ref(&barrier_to_src),
        );
    }

    // Copy to buffer
    let region = vk::BufferImageCopy::builder()
        .buffer_offset(0)
        .buffer_row_length(0) // tightly packed
        .buffer_image_height(0)
        .image_subresource(vk::ImageSubresourceLayers { aspect_mask: vk::ImageAspectFlags::COLOR, mip_level: 0, base_array_layer: 0, layer_count: 1 })
        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .image_extent(vk::Extent3D { width, height, depth: 1 });
    unsafe { ctx.device.cmd_copy_image_to_buffer(cmd_buf, image, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, buffer, std::slice::from_ref(&region)); }

    unsafe { ctx.device.end_command_buffer(cmd_buf)? };

    // Submit and wait
    let submit = vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&cmd_buf));
    unsafe {
        ctx.device.queue_submit(ctx.graphics_queue, std::slice::from_ref(&submit), vk::Fence::null())?;
        ctx.device.queue_wait_idle(ctx.graphics_queue)?;
    }

    // Read back buffer
    let ptr = unsafe { ctx.device.map_memory(buffer_mem, 0, buffer_size, vk::MemoryMapFlags::empty())? } as *const u8;
    let mut pixels = vec![0u8; buffer_size as usize];
    unsafe { std::ptr::copy_nonoverlapping(ptr, pixels.as_mut_ptr(), pixels.len()); }
    unsafe { ctx.device.unmap_memory(buffer_mem) };

    // Cleanup Vulkan objects
    unsafe {
        ctx.device.destroy_pipeline(pipeline, None);
        ctx.device.destroy_pipeline_layout(pipeline_layout, None);
        ctx.device.destroy_shader_module(vert, None);
        ctx.device.destroy_shader_module(frag, None);
        ctx.device.destroy_image_view(image_view, None);
        ctx.device.destroy_image(image, None);
        ctx.device.free_memory(image_mem, None);
        ctx.device.destroy_buffer(buffer, None);
        ctx.device.free_memory(buffer_mem, None);
        ctx.device.destroy_command_pool(cmd_pool, None);
    }

    Ok(pixels)
}

pub struct GBufferImages {
    pub albedo: (ash::vk::Image, ash::vk::DeviceMemory, ash::vk::ImageView),
    pub normal: (ash::vk::Image, ash::vk::DeviceMemory, ash::vk::ImageView),
    pub depth: (ash::vk::Image, ash::vk::DeviceMemory, ash::vk::ImageView),
}

impl GBufferImages {
    fn destroy(self, device: &ash::Device) {
        unsafe {
            device.destroy_image_view(self.albedo.2, None);
            device.destroy_image(self.albedo.0, None);
            device.free_memory(self.albedo.1, None);

            device.destroy_image_view(self.normal.2, None);
            device.destroy_image(self.normal.0, None);
            device.free_memory(self.normal.1, None);

            device.destroy_image_view(self.depth.2, None);
            device.destroy_image(self.depth.0, None);
            device.free_memory(self.depth.1, None);
        }
    }
}

fn create_image_2d(ctx: &VkContext, width: u32, height: u32, format: ash::vk::Format, usage: ash::vk::ImageUsageFlags, aspect: ash::vk::ImageAspectFlags) -> Result<(ash::vk::Image, ash::vk::DeviceMemory, ash::vk::ImageView)> {
    use ash::vk as vk;
    let image_ci = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .format(format)
        .extent(vk::Extent3D { width, height, depth: 1 })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(usage)
        .initial_layout(vk::ImageLayout::UNDEFINED);
    let image = unsafe { ctx.device.create_image(&image_ci, None)? };
    let mem_reqs = unsafe { ctx.device.get_image_memory_requirements(image) };
    let mem_type = find_memory_type(&ctx.instance, ctx.pdevice, mem_reqs.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL)?;
    let alloc = vk::MemoryAllocateInfo::builder().allocation_size(mem_reqs.size).memory_type_index(mem_type);
    let image_mem = unsafe { ctx.device.allocate_memory(&alloc, None)? };
    unsafe { ctx.device.bind_image_memory(image, image_mem, 0)? };
    let view_ci = vk::ImageViewCreateInfo::builder()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: aspect, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
    let view = unsafe { ctx.device.create_image_view(&view_ci, None)? };
    Ok((image, image_mem, view))
}

pub fn render_gbuffer_offscreen(ctx: &VkContext, width: u32, height: u32) -> Result<(Vec<u8>, Vec<u8>)> {
    use ash::vk as vk;

    let albedo_format = vk::Format::R8G8B8A8_UNORM;
    let normal_format = vk::Format::R8G8B8A8_UNORM;
    let material_format = vk::Format::R8_UINT;
    let depth_format = vk::Format::D32_SFLOAT;

    let albedo = create_image_2d(ctx, width, height, albedo_format, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC, vk::ImageAspectFlags::COLOR)?;
    let normal = create_image_2d(ctx, width, height, normal_format, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC, vk::ImageAspectFlags::COLOR)?;
    let material = create_image_2d(ctx, width, height, material_format, vk::ImageUsageFlags::COLOR_ATTACHMENT, vk::ImageAspectFlags::COLOR)?;
    let depth = create_image_2d(ctx, width, height, depth_format, vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT, vk::ImageAspectFlags::DEPTH)?;
    let gb = GBufferImages { albedo, normal, depth };

    // Pipeline setup
    let vert = create_shader_module(&ctx.device, GBUFFER_VERT_SPV)?;
    let frag = create_shader_module(&ctx.device, GBUFFER_FRAG_SPV)?;

    let stages = [
        vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::VERTEX).module(vert).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
        vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::FRAGMENT).module(frag).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
    ];
    let layout = vk::PipelineLayoutCreateInfo::builder();
    let pipeline_layout = unsafe { ctx.device.create_pipeline_layout(&layout, None)? };
    let ia = vk::PipelineInputAssemblyStateCreateInfo::builder().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    let vp = vk::PipelineViewportStateCreateInfo::builder().viewport_count(1).scissor_count(1);
    let rs = vk::PipelineRasterizationStateCreateInfo::builder().polygon_mode(vk::PolygonMode::FILL).cull_mode(vk::CullModeFlags::NONE).front_face(vk::FrontFace::COUNTER_CLOCKWISE).line_width(1.0);
    let ms = vk::PipelineMultisampleStateCreateInfo::builder().rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let cb_mask = vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A;
    let cba = [
        vk::PipelineColorBlendAttachmentState::builder().color_write_mask(cb_mask).blend_enable(false).build(),
        vk::PipelineColorBlendAttachmentState::builder().color_write_mask(cb_mask).blend_enable(false).build(),
        vk::PipelineColorBlendAttachmentState::builder().color_write_mask(cb_mask).blend_enable(false).build(),
    ];
    let cb = vk::PipelineColorBlendStateCreateInfo::builder().attachments(&cba);
    let ds = vk::PipelineDepthStencilStateCreateInfo::builder().depth_test_enable(true).depth_write_enable(true).depth_compare_op(vk::CompareOp::LESS_OR_EQUAL);
    let dyn_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dyn_state = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dyn_states);

    let color_formats = [albedo_format, normal_format, material_format];
    let mut rendering_info = vk::PipelineRenderingCreateInfo::builder()
        .color_attachment_formats(&color_formats)
        .depth_attachment_format(depth_format);
    let vi = vk::PipelineVertexInputStateCreateInfo::default();
    let vpci = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&stages)
        .vertex_input_state(&vi)
        .input_assembly_state(&ia)
        .viewport_state(&vp)
        .rasterization_state(&rs)
        .multisample_state(&ms)
        .depth_stencil_state(&ds)
        .color_blend_state(&cb)
        .dynamic_state(&dyn_state)
        .layout(pipeline_layout)
        .push_next(&mut rendering_info);
    let pipeline = unsafe { ctx.device.create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&vpci), None) }
        .map_err(|e| anyhow!("pipeline creation failed: {:?}", e.1))?[0];

    // Command pool + buffer
    let pool_ci = vk::CommandPoolCreateInfo::builder().queue_family_index(ctx.graphics_queue_family);
    let cmd_pool = unsafe { ctx.device.create_command_pool(&pool_ci, None)? };
    let alloc_ci = vk::CommandBufferAllocateInfo::builder().command_pool(cmd_pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1);
    let cmd_buf = unsafe { ctx.device.allocate_command_buffers(&alloc_ci)? }[0];
    let begin = vk::CommandBufferBeginInfo::builder();
    unsafe { ctx.device.begin_command_buffer(cmd_buf, &begin)? };

    // Transition color+depth to attachment layouts
    let to_color = |image| vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
        .build();
    let to_depth = |image| vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::DEPTH, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
        .build();
    let barriers = [to_color(gb.albedo.0), to_color(gb.normal.0), to_color(material.0), to_depth(gb.depth.0)];
    unsafe {
        ctx.device.cmd_pipeline_barrier(
            cmd_buf,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &barriers,
        );
    }

    // Begin rendering with three color attachments and one depth
    let clear_albedo = vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } };
    let clear_normal = vk::ClearValue { color: vk::ClearColorValue { float32: [0.5, 0.5, 1.0, 1.0] } };
    let clear_depth = vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } };
    let att0 = vk::RenderingAttachmentInfo::builder().image_view(gb.albedo.2).image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::STORE).clear_value(clear_albedo).build();
    let att1 = vk::RenderingAttachmentInfo::builder().image_view(gb.normal.2).image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::STORE).clear_value(clear_normal).build();
    let clear_mat = vk::ClearValue { color: vk::ClearColorValue { uint32: [0, 0, 0, 0] } };
    let att2 = vk::RenderingAttachmentInfo::builder().image_view(material.2).image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::STORE).clear_value(clear_mat).build();
    let color_atts = [att0, att1, att2];
    let depth_att = vk::RenderingAttachmentInfo::builder().image_view(gb.depth.2).image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::DONT_CARE).clear_value(clear_depth);
    let render_info = vk::RenderingInfo::builder()
        .render_area(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } })
        .layer_count(1)
        .color_attachments(&color_atts)
        .depth_attachment(&depth_att);
    unsafe {
        ctx.device.cmd_begin_rendering(cmd_buf, &render_info);
        // viewport/scissor
        let viewport = vk::Viewport { x: 0.0, y: 0.0, width: width as f32, height: height as f32, min_depth: 0.0, max_depth: 1.0 };
        let scissor = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } };
        ctx.device.cmd_set_viewport(cmd_buf, 0, std::slice::from_ref(&viewport));
        ctx.device.cmd_set_scissor(cmd_buf, 0, std::slice::from_ref(&scissor));
        ctx.device.cmd_bind_pipeline(cmd_buf, vk::PipelineBindPoint::GRAPHICS, pipeline);
        ctx.device.cmd_draw(cmd_buf, 3, 1, 0, 0);
        ctx.device.cmd_end_rendering(cmd_buf);
    }

    // Transition colors to TRANSFER_SRC and copy to host buffers
    let to_src = |image| vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
        .build();
    let barriers2 = [to_src(gb.albedo.0), to_src(gb.normal.0)];
    unsafe {
        ctx.device.cmd_pipeline_barrier(
            cmd_buf,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &barriers2,
        );
    }

    // Create staging buffers and copy
    let buf_size = (width as usize * height as usize * 4) as u64;
    let make_buffer = |usage: vk::BufferUsageFlags| -> Result<(vk::Buffer, vk::DeviceMemory)> {
        let ci = vk::BufferCreateInfo::builder().size(buf_size).usage(usage).sharing_mode(vk::SharingMode::EXCLUSIVE);
        let b = unsafe { ctx.device.create_buffer(&ci, None)? };
        let req = unsafe { ctx.device.get_buffer_memory_requirements(b) };
        let mt = find_memory_type(&ctx.instance, ctx.pdevice, req.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)?;
        let ai = vk::MemoryAllocateInfo::builder().allocation_size(req.size).memory_type_index(mt);
        let mem = unsafe { ctx.device.allocate_memory(&ai, None)? };
        unsafe { ctx.device.bind_buffer_memory(b, mem, 0)? };
        Ok((b, mem))
    };
    let (buf_a, mem_a) = make_buffer(vk::BufferUsageFlags::TRANSFER_DST)?;
    let (buf_n, mem_n) = make_buffer(vk::BufferUsageFlags::TRANSFER_DST)?;

    let region = vk::BufferImageCopy::builder()
        .buffer_offset(0)
        .buffer_row_length(0)
        .buffer_image_height(0)
        .image_subresource(vk::ImageSubresourceLayers { aspect_mask: vk::ImageAspectFlags::COLOR, mip_level: 0, base_array_layer: 0, layer_count: 1 })
        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .image_extent(vk::Extent3D { width, height, depth: 1 });
    unsafe {
        ctx.device.cmd_copy_image_to_buffer(cmd_buf, gb.albedo.0, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, buf_a, std::slice::from_ref(&region));
        ctx.device.cmd_copy_image_to_buffer(cmd_buf, gb.normal.0, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, buf_n, std::slice::from_ref(&region));
    }

    unsafe { ctx.device.end_command_buffer(cmd_buf)? };
    let submit = vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&cmd_buf));
    unsafe {
        ctx.device.queue_submit(ctx.graphics_queue, std::slice::from_ref(&submit), vk::Fence::null())?;
        ctx.device.queue_wait_idle(ctx.graphics_queue)?;
    }

    // Map and read back
    let read_back = |mem: vk::DeviceMemory| -> Result<Vec<u8>> {
        let ptr = unsafe { ctx.device.map_memory(mem, 0, buf_size, vk::MemoryMapFlags::empty())? } as *const u8;
        let mut v = vec![0u8; buf_size as usize];
        unsafe { std::ptr::copy_nonoverlapping(ptr, v.as_mut_ptr(), v.len()); }
        unsafe { ctx.device.unmap_memory(mem) };
        Ok(v)
    };
    let albedo_pixels = read_back(mem_a)?;
    let normal_pixels = read_back(mem_n)?;

    // Cleanup
    unsafe {
        ctx.device.destroy_pipeline(pipeline, None);
        ctx.device.destroy_pipeline_layout(pipeline_layout, None);
        ctx.device.destroy_shader_module(vert, None);
        ctx.device.destroy_shader_module(frag, None);
        ctx.device.destroy_command_pool(cmd_pool, None);
        ctx.device.destroy_buffer(buf_a, None);
        ctx.device.destroy_buffer(buf_n, None);
        ctx.device.free_memory(mem_a, None);
        ctx.device.free_memory(mem_n, None);
        ctx.device.destroy_image_view(material.2, None);
        ctx.device.destroy_image(material.0, None);
        ctx.device.free_memory(material.1, None);
    }
    gb.destroy(&ctx.device);

    Ok((albedo_pixels, normal_pixels))
}

pub fn render_toon_from_gbuffer(ctx: &VkContext, width: u32, height: u32, style: &ToonStyle) -> Result<Vec<u8>> {
    use ash::vk as vk;

    // 1) Create G-buffer with SAMPLED usage
    let albedo_format = vk::Format::R8G8B8A8_UNORM;
    let normal_format = vk::Format::R8G8B8A8_UNORM;
    let material_format = vk::Format::R8_UINT;
    let depth_format = vk::Format::D32_SFLOAT;
    let albedo = create_image_2d(ctx, width, height, albedo_format, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED, vk::ImageAspectFlags::COLOR)?;
    let normal = create_image_2d(ctx, width, height, normal_format, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED, vk::ImageAspectFlags::COLOR)?;
    let material = create_image_2d(ctx, width, height, material_format, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED, vk::ImageAspectFlags::COLOR)?;
    let depth = create_image_2d(ctx, width, height, depth_format, vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT, vk::ImageAspectFlags::DEPTH)?;
    let gb = GBufferImages { albedo, normal, depth };

    // 2) Render G-buffer
    {
        let vert = create_shader_module(&ctx.device, GBUFFER_VERT_SPV)?;
        let frag = create_shader_module(&ctx.device, GBUFFER_FRAG_SPV)?;
        let stages = [
            vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::VERTEX).module(vert).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
            vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::FRAGMENT).module(frag).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
        ];
        let layout = vk::PipelineLayoutCreateInfo::builder();
        let pipeline_layout = unsafe { ctx.device.create_pipeline_layout(&layout, None)? };
        let ia = vk::PipelineInputAssemblyStateCreateInfo::builder().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let vp = vk::PipelineViewportStateCreateInfo::builder().viewport_count(1).scissor_count(1);
        let rs = vk::PipelineRasterizationStateCreateInfo::builder().polygon_mode(vk::PolygonMode::FILL).cull_mode(vk::CullModeFlags::NONE).front_face(vk::FrontFace::COUNTER_CLOCKWISE).line_width(1.0);
        let ms = vk::PipelineMultisampleStateCreateInfo::builder().rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let cb_mask = vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A;
        let cba = [
            vk::PipelineColorBlendAttachmentState::builder().color_write_mask(cb_mask).blend_enable(false).build(),
            vk::PipelineColorBlendAttachmentState::builder().color_write_mask(cb_mask).blend_enable(false).build(),
            vk::PipelineColorBlendAttachmentState::builder().color_write_mask(cb_mask).blend_enable(false).build(),
        ];
        let cb = vk::PipelineColorBlendStateCreateInfo::builder().attachments(&cba);
        let ds = vk::PipelineDepthStencilStateCreateInfo::builder().depth_test_enable(true).depth_write_enable(true).depth_compare_op(vk::CompareOp::LESS_OR_EQUAL);
        let dyn_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dyn_state = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dyn_states);
        let color_formats = [albedo_format, normal_format, material_format];
        let mut rendering_info = vk::PipelineRenderingCreateInfo::builder()
            .color_attachment_formats(&color_formats)
            .depth_attachment_format(depth_format);
        let vi = vk::PipelineVertexInputStateCreateInfo::default();
        let vpci = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&stages)
            .vertex_input_state(&vi)
            .input_assembly_state(&ia)
            .viewport_state(&vp)
            .rasterization_state(&rs)
            .multisample_state(&ms)
            .depth_stencil_state(&ds)
            .color_blend_state(&cb)
            .dynamic_state(&dyn_state)
            .layout(pipeline_layout)
            .push_next(&mut rendering_info);
        let pipeline = unsafe { ctx.device.create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&vpci), None) }
            .map_err(|e| anyhow!("pipeline creation failed: {:?}", e.1))?[0];

        // Record pass
        let pool_ci = vk::CommandPoolCreateInfo::builder().queue_family_index(ctx.graphics_queue_family);
        let cmd_pool = unsafe { ctx.device.create_command_pool(&pool_ci, None)? };
        let alloc_ci = vk::CommandBufferAllocateInfo::builder().command_pool(cmd_pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1);
        let cmd_buf = unsafe { ctx.device.allocate_command_buffers(&alloc_ci)? }[0];
        let begin = vk::CommandBufferBeginInfo::builder();
        unsafe { ctx.device.begin_command_buffer(cmd_buf, &begin)? };

        // Transitions
        let to_color = |image| vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
            .build();
        let to_depth = |image| vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::DEPTH, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
            .build();
        let barriers = [to_color(gb.albedo.0), to_color(gb.normal.0), to_color(material.0), to_depth(gb.depth.0)];
        unsafe {
            ctx.device.cmd_pipeline_barrier(
                cmd_buf,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &barriers,
            );
        }

        let clear_albedo = vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } };
        let clear_normal = vk::ClearValue { color: vk::ClearColorValue { float32: [0.5, 0.5, 1.0, 1.0] } };
        let clear_depth = vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } };
        let att0 = vk::RenderingAttachmentInfo::builder().image_view(gb.albedo.2).image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::STORE).clear_value(clear_albedo).build();
        let att1 = vk::RenderingAttachmentInfo::builder().image_view(gb.normal.2).image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::STORE).clear_value(clear_normal).build();
        let clear_mat = vk::ClearValue { color: vk::ClearColorValue { uint32: [0, 0, 0, 0] } };
        let att2 = vk::RenderingAttachmentInfo::builder().image_view(material.2).image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::STORE).clear_value(clear_mat).build();
        let color_atts = [att0, att1, att2];
        let depth_att = vk::RenderingAttachmentInfo::builder().image_view(gb.depth.2).image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::DONT_CARE).clear_value(clear_depth);
        let render_info = vk::RenderingInfo::builder()
            .render_area(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } })
            .layer_count(1)
            .color_attachments(&color_atts)
            .depth_attachment(&depth_att);
        unsafe {
            ctx.device.cmd_begin_rendering(cmd_buf, &render_info);
            let viewport = vk::Viewport { x: 0.0, y: 0.0, width: width as f32, height: height as f32, min_depth: 0.0, max_depth: 1.0 };
            let scissor = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } };
            ctx.device.cmd_set_viewport(cmd_buf, 0, std::slice::from_ref(&viewport));
            ctx.device.cmd_set_scissor(cmd_buf, 0, std::slice::from_ref(&scissor));
            ctx.device.cmd_bind_pipeline(cmd_buf, vk::PipelineBindPoint::GRAPHICS, pipeline);
            ctx.device.cmd_draw(cmd_buf, 3, 1, 0, 0);
            ctx.device.cmd_end_rendering(cmd_buf);
        }
        unsafe {
            ctx.device.end_command_buffer(cmd_buf)?;
            let submit = vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&cmd_buf));
            ctx.device.queue_submit(ctx.graphics_queue, std::slice::from_ref(&submit), vk::Fence::null())?;
            ctx.device.queue_wait_idle(ctx.graphics_queue)?;
            ctx.device.destroy_pipeline(pipeline, None);
            ctx.device.destroy_pipeline_layout(pipeline_layout, None);
            ctx.device.destroy_shader_module(vert, None);
            ctx.device.destroy_shader_module(frag, None);
            ctx.device.destroy_command_pool(cmd_pool, None);
        }
    }

    // 3) Transition albedo/normal to SHADER_READ_ONLY_OPTIMAL
    {
        let pool_ci = vk::CommandPoolCreateInfo::builder().queue_family_index(ctx.graphics_queue_family);
        let cmd_pool = unsafe { ctx.device.create_command_pool(&pool_ci, None)? };
        let alloc_ci = vk::CommandBufferAllocateInfo::builder().command_pool(cmd_pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1);
        let cmd_buf = unsafe { ctx.device.allocate_command_buffers(&alloc_ci)? }[0];
        let begin = vk::CommandBufferBeginInfo::builder();
        unsafe { ctx.device.begin_command_buffer(cmd_buf, &begin)? };
        let to_read = |image| vk::ImageMemoryBarrier::builder()
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
            .build();
        let barriers = [to_read(gb.albedo.0), to_read(gb.normal.0)];
        unsafe {
            ctx.device.cmd_pipeline_barrier(
                cmd_buf,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &barriers,
            );
            ctx.device.end_command_buffer(cmd_buf)?;
            let submit = vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&cmd_buf));
            ctx.device.queue_submit(ctx.graphics_queue, std::slice::from_ref(&submit), vk::Fence::null())?;
            ctx.device.queue_wait_idle(ctx.graphics_queue)?;
            ctx.device.destroy_command_pool(cmd_pool, None);
        }
    }

    // 4) Create sampler and descriptor set with albedo/normal
    let sampler_ci = vk::SamplerCreateInfo::builder().mag_filter(vk::Filter::LINEAR).min_filter(vk::Filter::LINEAR).address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE).address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE).address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE);
    let sampler = unsafe { ctx.device.create_sampler(&sampler_ci, None)? };
    let bindings = [
        vk::DescriptorSetLayoutBinding::builder().binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT).build(),
        vk::DescriptorSetLayoutBinding::builder().binding(1).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT).build(),
        vk::DescriptorSetLayoutBinding::builder().binding(2).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT).build(),
        vk::DescriptorSetLayoutBinding::builder().binding(3).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT).build(),
    ];
    let dsl_ci = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);
    let dsl = unsafe { ctx.device.create_descriptor_set_layout(&dsl_ci, None)? };
    let pc_range = vk::PushConstantRange::builder()
        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
        .offset(0)
        .size(48)
        .build();
    let layout_ci = vk::PipelineLayoutCreateInfo::builder()
        .set_layouts(std::slice::from_ref(&dsl))
        .push_constant_ranges(std::slice::from_ref(&pc_range));
    let pipeline_layout = unsafe { ctx.device.create_pipeline_layout(&layout_ci, None)? };
    let pool_sizes = [
        vk::DescriptorPoolSize { ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER, descriptor_count: 3 },
        vk::DescriptorPoolSize { ty: vk::DescriptorType::UNIFORM_BUFFER, descriptor_count: 1 },
    ];
    let dp_ci = vk::DescriptorPoolCreateInfo::builder().max_sets(1).pool_sizes(&pool_sizes);
    let dpool = unsafe { ctx.device.create_descriptor_pool(&dp_ci, None)? };
    let alloc_info = vk::DescriptorSetAllocateInfo::builder().descriptor_pool(dpool).set_layouts(std::slice::from_ref(&dsl));
    let dset = unsafe { ctx.device.allocate_descriptor_sets(&alloc_info)? }[0];
    let info_albedo = vk::DescriptorImageInfo::builder()
        .sampler(sampler)
        .image_view(gb.albedo.2)
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .build();
    let info_normal = vk::DescriptorImageInfo::builder()
        .sampler(sampler)
        .image_view(gb.normal.2)
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .build();
    let writes = [
        vk::WriteDescriptorSet::builder()
            .dst_set(dset)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&info_albedo))
            .build(),
        vk::WriteDescriptorSet::builder()
            .dst_set(dset)
            .dst_binding(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&info_normal))
            .build(),
    ];
    unsafe { ctx.device.update_descriptor_sets(&writes, &[]) };

    // 5) Create toon output image
    let out_format = vk::Format::R8G8B8A8_UNORM;
    let (out_img, out_mem, out_view) = create_image_2d(ctx, width, height, out_format, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC, vk::ImageAspectFlags::COLOR)?;

    // 6) Create toon pipeline
    let vmod = create_shader_module(&ctx.device, FSQ_VERT_SPV)?;
    let fmod = create_shader_module(&ctx.device, TOON_GBUFFER_FRAG_SPV)?;
    let stages = [
        vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::VERTEX).module(vmod).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
        vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::FRAGMENT).module(fmod).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
    ];
    let ia = vk::PipelineInputAssemblyStateCreateInfo::builder().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    let vp = vk::PipelineViewportStateCreateInfo::builder().viewport_count(1).scissor_count(1);
    let rs = vk::PipelineRasterizationStateCreateInfo::builder().polygon_mode(vk::PolygonMode::FILL).cull_mode(vk::CullModeFlags::NONE).front_face(vk::FrontFace::COUNTER_CLOCKWISE).line_width(1.0);
    let ms = vk::PipelineMultisampleStateCreateInfo::builder().rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let cb_mask = vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A;
    let cba = vk::PipelineColorBlendAttachmentState::builder().color_write_mask(cb_mask).blend_enable(false).build();
    let cb = vk::PipelineColorBlendStateCreateInfo::builder().attachments(std::slice::from_ref(&cba));
    let ds = vk::PipelineDepthStencilStateCreateInfo::builder().depth_test_enable(false).depth_write_enable(false);
    let dyn_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dyn_state = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dyn_states);
    let mut rendering_info = vk::PipelineRenderingCreateInfo::builder().color_attachment_formats(std::slice::from_ref(&out_format));
    let vi = vk::PipelineVertexInputStateCreateInfo::default();
    let vpci = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&stages)
        .vertex_input_state(&vi)
        .input_assembly_state(&ia)
        .viewport_state(&vp)
        .rasterization_state(&rs)
        .multisample_state(&ms)
        .depth_stencil_state(&ds)
        .color_blend_state(&cb)
        .dynamic_state(&dyn_state)
        .layout(pipeline_layout)
        .push_next(&mut rendering_info);
    let pipeline = unsafe { ctx.device.create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&vpci), None) }
        .map_err(|e| anyhow!("pipeline creation failed: {:?}", e.1))?[0];

    // 7) Transition output image and record draw
    let pool_ci = vk::CommandPoolCreateInfo::builder().queue_family_index(ctx.graphics_queue_family);
    let cmd_pool = unsafe { ctx.device.create_command_pool(&pool_ci, None)? };
    let alloc_ci = vk::CommandBufferAllocateInfo::builder().command_pool(cmd_pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1);
    let cmd_buf = unsafe { ctx.device.allocate_command_buffers(&alloc_ci)? }[0];
    let begin = vk::CommandBufferBeginInfo::builder();
    unsafe { ctx.device.begin_command_buffer(cmd_buf, &begin)? };

    let barrier_to_color = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .image(out_img)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
    unsafe {
        ctx.device.cmd_pipeline_barrier(
            cmd_buf,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            std::slice::from_ref(&barrier_to_color),
        );
    }

    let clear = vk::ClearValue { color: vk::ClearColorValue { float32: [0.04, 0.04, 0.06, 1.0] } };
    let att = vk::RenderingAttachmentInfo::builder()
        .image_view(out_view)
        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .clear_value(clear);
    let render_info = vk::RenderingInfo::builder()
        .render_area(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } })
        .layer_count(1)
        .color_attachments(std::slice::from_ref(&att));
    unsafe {
        ctx.device.cmd_begin_rendering(cmd_buf, &render_info);
        let viewport = vk::Viewport { x: 0.0, y: 0.0, width: width as f32, height: height as f32, min_depth: 0.0, max_depth: 1.0 };
        let scissor = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } };
        ctx.device.cmd_set_viewport(cmd_buf, 0, std::slice::from_ref(&viewport));
        ctx.device.cmd_set_scissor(cmd_buf, 0, std::slice::from_ref(&scissor));
        ctx.device.cmd_bind_pipeline(cmd_buf, vk::PipelineBindPoint::GRAPHICS, pipeline);
        ctx.device.cmd_bind_descriptor_sets(cmd_buf, vk::PipelineBindPoint::GRAPHICS, pipeline_layout, 0, std::slice::from_ref(&dset), &[]);
        #[repr(C)]
        struct ToonPC { data: [f32; 12] }
        let pc = ToonPC { data: [
            0.60,   // shadowThreshold
            -1.0,   // midThreshold (disabled)
            0.20,   // rimStrength
            0.35,   // rimWidth
            0.05,   // bandSoftness
            -6.0,   // hueShiftShadowDeg (slightly cooler)
            6.0,    // hueShiftLightDeg  (slightly warmer)
            0.95,   // satScaleShadow
            1.05,   // satScaleLight
            0.86,   // specThreshold
            0.22,   // specIntensity
            0.0,    // _pad
        ]};
        let bytes = std::slice::from_raw_parts((&pc as *const ToonPC) as *const u8, std::mem::size_of::<ToonPC>());
        ctx.device.cmd_push_constants(cmd_buf, pipeline_layout, vk::ShaderStageFlags::FRAGMENT, 0, bytes);
        ctx.device.cmd_draw(cmd_buf, 3, 1, 0, 0);
        ctx.device.cmd_end_rendering(cmd_buf);
    }

    // Copy output to CPU buffer
    let buf_size = (width as usize * height as usize * 4) as u64;
    let buf_ci = vk::BufferCreateInfo::builder().size(buf_size).usage(vk::BufferUsageFlags::TRANSFER_DST).sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = unsafe { ctx.device.create_buffer(&buf_ci, None)? };
    let req = unsafe { ctx.device.get_buffer_memory_requirements(buffer) };
    let mt = find_memory_type(&ctx.instance, ctx.pdevice, req.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)?;
    let ai = vk::MemoryAllocateInfo::builder().allocation_size(req.size).memory_type_index(mt);
    let buffer_mem = unsafe { ctx.device.allocate_memory(&ai, None)? };
    unsafe { ctx.device.bind_buffer_memory(buffer, buffer_mem, 0)? };

    let barrier_to_src = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .image(out_img)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
    unsafe {
        ctx.device.cmd_pipeline_barrier(
            cmd_buf,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            std::slice::from_ref(&barrier_to_src),
        );
        let region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers { aspect_mask: vk::ImageAspectFlags::COLOR, mip_level: 0, base_array_layer: 0, layer_count: 1 })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D { width, height, depth: 1 });
        ctx.device.cmd_copy_image_to_buffer(cmd_buf, out_img, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, buffer, std::slice::from_ref(&region));
        ctx.device.end_command_buffer(cmd_buf)?;
        let submit = vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&cmd_buf));
        ctx.device.queue_submit(ctx.graphics_queue, std::slice::from_ref(&submit), vk::Fence::null())?;
        ctx.device.queue_wait_idle(ctx.graphics_queue)?;
    }

    // Read back
    let ptr = unsafe { ctx.device.map_memory(buffer_mem, 0, buf_size, vk::MemoryMapFlags::empty())? } as *const u8;
    let mut pixels = vec![0u8; buf_size as usize];
    unsafe { std::ptr::copy_nonoverlapping(ptr, pixels.as_mut_ptr(), pixels.len()); }
    unsafe { ctx.device.unmap_memory(buffer_mem) };

    // Cleanup
    unsafe {
        ctx.device.destroy_pipeline(pipeline, None);
        ctx.device.destroy_pipeline_layout(pipeline_layout, None);
        ctx.device.destroy_descriptor_pool(dpool, None);
        ctx.device.destroy_descriptor_set_layout(dsl, None);
        ctx.device.destroy_sampler(sampler, None);
        ctx.device.destroy_shader_module(vmod, None);
        ctx.device.destroy_shader_module(fmod, None);
        ctx.device.destroy_command_pool(cmd_pool, None);
        ctx.device.destroy_buffer(buffer, None);
        ctx.device.free_memory(buffer_mem, None);
        ctx.device.destroy_image_view(out_view, None);
        ctx.device.destroy_image(out_img, None);
        ctx.device.free_memory(out_mem, None);
    }
    gb.destroy(&ctx.device);

    Ok(pixels)
}

pub fn render_toon_from_mesh(ctx: &VkContext, width: u32, height: u32, style: &ToonStyle, outline_width_px: Option<f32>) -> Result<Vec<u8>> {
    use ash::vk as vk;
    use crate::render::mesh::{generate_uv_sphere, Vertex};

    // Generate a UV-sphere mesh
    let (verts, inds) = generate_uv_sphere(0.8, 32, 64);

    // Create HOST_VISIBLE vertex and index buffers and upload data
    let vb_size = (std::mem::size_of::<Vertex>() * verts.len()) as u64;
    let ib_size = (std::mem::size_of::<u32>() * inds.len()) as u64;
    let host_props = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
    let vb_ci = vk::BufferCreateInfo::builder().size(vb_size).usage(vk::BufferUsageFlags::VERTEX_BUFFER).sharing_mode(vk::SharingMode::EXCLUSIVE);
    let ib_ci = vk::BufferCreateInfo::builder().size(ib_size).usage(vk::BufferUsageFlags::INDEX_BUFFER).sharing_mode(vk::SharingMode::EXCLUSIVE);
    let vb = unsafe { ctx.device.create_buffer(&vb_ci, None)? };
    let ib = unsafe { ctx.device.create_buffer(&ib_ci, None)? };
    let vb_req = unsafe { ctx.device.get_buffer_memory_requirements(vb) };
    let ib_req = unsafe { ctx.device.get_buffer_memory_requirements(ib) };
    let vb_type = find_memory_type(&ctx.instance, ctx.pdevice, vb_req.memory_type_bits, host_props)?;
    let ib_type = find_memory_type(&ctx.instance, ctx.pdevice, ib_req.memory_type_bits, host_props)?;
    let vb_alloc = vk::MemoryAllocateInfo::builder().allocation_size(vb_req.size).memory_type_index(vb_type);
    let ib_alloc = vk::MemoryAllocateInfo::builder().allocation_size(ib_req.size).memory_type_index(ib_type);
    let vb_mem = unsafe { ctx.device.allocate_memory(&vb_alloc, None)? };
    let ib_mem = unsafe { ctx.device.allocate_memory(&ib_alloc, None)? };
    unsafe { ctx.device.bind_buffer_memory(vb, vb_mem, 0)? };
    unsafe { ctx.device.bind_buffer_memory(ib, ib_mem, 0)? };
    unsafe {
        let p = ctx.device.map_memory(vb_mem, 0, vb_size, vk::MemoryMapFlags::empty())? as *mut Vertex;
        std::ptr::copy_nonoverlapping(verts.as_ptr(), p, verts.len());
        ctx.device.unmap_memory(vb_mem);
        let p = ctx.device.map_memory(ib_mem, 0, ib_size, vk::MemoryMapFlags::empty())? as *mut u32;
        std::ptr::copy_nonoverlapping(inds.as_ptr(), p, inds.len());
        ctx.device.unmap_memory(ib_mem);
    }

    // Create G-buffer attachments with SAMPLED so we can use them in the toon pass
    let albedo_format = vk::Format::R8G8B8A8_UNORM;
    let normal_format = vk::Format::R8G8B8A8_UNORM;
    let material_format = vk::Format::R8_UINT;
    let depth_format = vk::Format::D32_SFLOAT;
    let albedo = create_image_2d(ctx, width, height, albedo_format, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED, vk::ImageAspectFlags::COLOR)?;
    let normal = create_image_2d(ctx, width, height, normal_format, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED, vk::ImageAspectFlags::COLOR)?;
    let material = create_image_2d(ctx, width, height, material_format, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED, vk::ImageAspectFlags::COLOR)?;
    let depth = create_image_2d(ctx, width, height, depth_format, vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT, vk::ImageAspectFlags::DEPTH)?;

    // Create mesh G-buffer pipeline
    let vmod = create_shader_module(&ctx.device, MESH_GBUFFER_VERT_SPV)?;
    let fmod = create_shader_module(&ctx.device, MESH_GBUFFER_FRAG_SPV)?;
    let stages = [
        vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::VERTEX).module(vmod).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
        vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::FRAGMENT).module(fmod).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
    ];
    let binding_desc = vk::VertexInputBindingDescription::builder().binding(0).stride(std::mem::size_of::<Vertex>() as u32).input_rate(vk::VertexInputRate::VERTEX).build();
    let attr_descs = [
        vk::VertexInputAttributeDescription::builder().location(0).binding(0).format(vk::Format::R32G32B32_SFLOAT).offset(0).build(),
        vk::VertexInputAttributeDescription::builder().location(1).binding(0).format(vk::Format::R32G32B32_SFLOAT).offset(12).build(),
    ];
    let vi = vk::PipelineVertexInputStateCreateInfo::builder().vertex_binding_descriptions(std::slice::from_ref(&binding_desc)).vertex_attribute_descriptions(&attr_descs);
    let ia = vk::PipelineInputAssemblyStateCreateInfo::builder().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    let vp = vk::PipelineViewportStateCreateInfo::builder().viewport_count(1).scissor_count(1);
    let rs = vk::PipelineRasterizationStateCreateInfo::builder().polygon_mode(vk::PolygonMode::FILL).cull_mode(vk::CullModeFlags::BACK).front_face(vk::FrontFace::COUNTER_CLOCKWISE).line_width(1.0);
    let ms = vk::PipelineMultisampleStateCreateInfo::builder().rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let cb_mask = vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A;
    let cba = [
        vk::PipelineColorBlendAttachmentState::builder().color_write_mask(cb_mask).blend_enable(false).build(),
        vk::PipelineColorBlendAttachmentState::builder().color_write_mask(cb_mask).blend_enable(false).build(),
        vk::PipelineColorBlendAttachmentState::builder().color_write_mask(cb_mask).blend_enable(false).build(),
    ];
    let cb = vk::PipelineColorBlendStateCreateInfo::builder().attachments(&cba);
    let ds = vk::PipelineDepthStencilStateCreateInfo::builder().depth_test_enable(true).depth_write_enable(true).depth_compare_op(vk::CompareOp::LESS_OR_EQUAL);
    let dyn_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dyn_state = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dyn_states);
    let color_formats = [albedo_format, normal_format, material_format];
    let mut rendering_info = vk::PipelineRenderingCreateInfo::builder().color_attachment_formats(&color_formats).depth_attachment_format(depth_format);
    let layout_ci = vk::PipelineLayoutCreateInfo::builder();
    let pipeline_layout = unsafe { ctx.device.create_pipeline_layout(&layout_ci, None)? };
    let gp_ci = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&stages)
        .vertex_input_state(&vi)
        .input_assembly_state(&ia)
        .viewport_state(&vp)
        .rasterization_state(&rs)
        .multisample_state(&ms)
        .depth_stencil_state(&ds)
        .color_blend_state(&cb)
        .dynamic_state(&dyn_state)
        .layout(pipeline_layout)
        .push_next(&mut rendering_info);
    let gbuf_pipeline = unsafe { ctx.device.create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&gp_ci), None) }
        .map_err(|e| anyhow!("pipeline creation failed: {:?}", e.1))?[0];

    // Record G-buffer pass
    let pool_ci = vk::CommandPoolCreateInfo::builder().queue_family_index(ctx.graphics_queue_family);
    let cmd_pool = unsafe { ctx.device.create_command_pool(&pool_ci, None)? };
    let alloc_ci = vk::CommandBufferAllocateInfo::builder().command_pool(cmd_pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1);
    let cmd_buf = unsafe { ctx.device.allocate_command_buffers(&alloc_ci)? }[0];
    let begin = vk::CommandBufferBeginInfo::builder();
    unsafe { ctx.device.begin_command_buffer(cmd_buf, &begin)? };
    let to_color = |image| vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
        .build();
    let to_depth = |image| vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::DEPTH, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
        .build();
    let barriers = [to_color(albedo.0), to_color(normal.0), to_color(material.0), to_depth(depth.0)];
    unsafe {
        ctx.device.cmd_pipeline_barrier(
            cmd_buf,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &barriers,
        );
    }
    let clear0 = vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } };
    let clear1 = vk::ClearValue { color: vk::ClearColorValue { float32: [0.5, 0.5, 1.0, 1.0] } };
    let att0 = vk::RenderingAttachmentInfo::builder().image_view(albedo.2).image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::STORE).clear_value(clear0).build();
    let att1 = vk::RenderingAttachmentInfo::builder().image_view(normal.2).image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::STORE).clear_value(clear1).build();
    let clear_mat = vk::ClearValue { color: vk::ClearColorValue { uint32: [2, 0, 0, 0] } };
    let att2 = vk::RenderingAttachmentInfo::builder().image_view(material.2).image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::STORE).clear_value(clear_mat).build();
    let color_atts = [att0, att1, att2];
    let depth_att = vk::RenderingAttachmentInfo::builder().image_view(depth.2).image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::STORE).clear_value(vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } });
    let render_info = vk::RenderingInfo::builder()
        .render_area(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } })
        .layer_count(1)
        .color_attachments(&color_atts)
        .depth_attachment(&depth_att);
    unsafe {
        ctx.device.cmd_begin_rendering(cmd_buf, &render_info);
        let viewport = vk::Viewport { x: 0.0, y: 0.0, width: width as f32, height: height as f32, min_depth: 0.0, max_depth: 1.0 };
        let scissor = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } };
        ctx.device.cmd_set_viewport(cmd_buf, 0, std::slice::from_ref(&viewport));
        ctx.device.cmd_set_scissor(cmd_buf, 0, std::slice::from_ref(&scissor));
        ctx.device.cmd_bind_pipeline(cmd_buf, vk::PipelineBindPoint::GRAPHICS, gbuf_pipeline);
        let vb_buffers = [vb];
        let vb_offsets = [0u64];
        ctx.device.cmd_bind_vertex_buffers(cmd_buf, 0, &vb_buffers, &vb_offsets);
        ctx.device.cmd_bind_index_buffer(cmd_buf, ib, 0, vk::IndexType::UINT32);
        ctx.device.cmd_draw_indexed(cmd_buf, inds.len() as u32, 1, 0, 0, 0);
        ctx.device.cmd_end_rendering(cmd_buf);
    }

    // Transition to SHADER_READ_ONLY for sampling in toon pass
    let to_read = |image| vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .dst_access_mask(vk::AccessFlags::SHADER_READ)
        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
        .build();
    let barriers2 = [to_read(albedo.0), to_read(normal.0), to_read(material.0)];
    unsafe {
        ctx.device.cmd_pipeline_barrier(
            cmd_buf,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &barriers2,
        );
        ctx.device.end_command_buffer(cmd_buf)?;
        let submit = vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&cmd_buf));
        ctx.device.queue_submit(ctx.graphics_queue, std::slice::from_ref(&submit), vk::Fence::null())?;
        ctx.device.queue_wait_idle(ctx.graphics_queue)?;
        ctx.device.destroy_command_pool(cmd_pool, None);
        ctx.device.destroy_pipeline(gbuf_pipeline, None);
        ctx.device.destroy_pipeline_layout(pipeline_layout, None);
        ctx.device.destroy_shader_module(vmod, None);
        ctx.device.destroy_shader_module(fmod, None);
    }

    // Create toon output pipeline sampling the G-buffer (reuse code from render_toon_from_gbuffer)
    let sampler_ci = vk::SamplerCreateInfo::builder().mag_filter(vk::Filter::LINEAR).min_filter(vk::Filter::LINEAR).address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE).address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE).address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE);
    let sampler = unsafe { ctx.device.create_sampler(&sampler_ci, None)? };
    let bindings = [
        vk::DescriptorSetLayoutBinding::builder().binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT).build(),
        vk::DescriptorSetLayoutBinding::builder().binding(1).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT).build(),
    ];
    let dsl_ci = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);
    let dsl = unsafe { ctx.device.create_descriptor_set_layout(&dsl_ci, None)? };
    let pc_range = vk::PushConstantRange::builder().stage_flags(vk::ShaderStageFlags::FRAGMENT).offset(0).size(48).build();
    let layout_ci = vk::PipelineLayoutCreateInfo::builder().set_layouts(std::slice::from_ref(&dsl)).push_constant_ranges(std::slice::from_ref(&pc_range));
    let pipeline_layout = unsafe { ctx.device.create_pipeline_layout(&layout_ci, None)? };
    let pool_sizes = [vk::DescriptorPoolSize { ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER, descriptor_count: 2 }];
    let dp_ci = vk::DescriptorPoolCreateInfo::builder().max_sets(1).pool_sizes(&pool_sizes);
    let dpool = unsafe { ctx.device.create_descriptor_pool(&dp_ci, None)? };
    let alloc_info = vk::DescriptorSetAllocateInfo::builder().descriptor_pool(dpool).set_layouts(std::slice::from_ref(&dsl));
    let dset = unsafe { ctx.device.allocate_descriptor_sets(&alloc_info)? }[0];
    let info_albedo = vk::DescriptorImageInfo::builder().sampler(sampler).image_view(albedo.2).image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL).build();
    let info_normal = vk::DescriptorImageInfo::builder().sampler(sampler).image_view(normal.2).image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL).build();
    let info_material = vk::DescriptorImageInfo::builder().sampler(sampler).image_view(material.2).image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL).build();
    // Create LUT UBO with default style (will be overridden by DNA path via CLI in mesh variant)
    let style = style;
    let lut_size = (8 * 3 * 16) as u64;
    let lut_buf_ci = vk::BufferCreateInfo::builder().size(lut_size).usage(vk::BufferUsageFlags::UNIFORM_BUFFER).sharing_mode(vk::SharingMode::EXCLUSIVE);
    let lut_buf = unsafe { ctx.device.create_buffer(&lut_buf_ci, None)? };
    let lut_req = unsafe { ctx.device.get_buffer_memory_requirements(lut_buf) };
    let lut_mt = find_memory_type(&ctx.instance, ctx.pdevice, lut_req.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)?;
    let lut_ai = vk::MemoryAllocateInfo::builder().allocation_size(lut_req.size).memory_type_index(lut_mt);
    let lut_mem = unsafe { ctx.device.allocate_memory(&lut_ai, None)? };
    unsafe { ctx.device.bind_buffer_memory(lut_buf, lut_mem, 0)? };
    unsafe {
        let ptr = ctx.device.map_memory(lut_mem, 0, lut_size, vk::MemoryMapFlags::empty())? as *mut f32;
        let slice = std::slice::from_raw_parts_mut(ptr, (lut_size/4) as usize);
        let mut idx = 0usize;
        for i in 0..8 { for j in 0..4 { slice[idx] = style.row0[i][j]; idx+=1; } }
        for i in 0..8 { for j in 0..4 { slice[idx] = style.row1[i][j]; idx+=1; } }
        for i in 0..8 { for j in 0..4 { slice[idx] = style.row2[i][j]; idx+=1; } }
        ctx.device.unmap_memory(lut_mem);
    }
    let lut_info = vk::DescriptorBufferInfo::builder().buffer(lut_buf).offset(0).range(lut_size).build();
    let writes = [
        vk::WriteDescriptorSet::builder().dst_set(dset).dst_binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(std::slice::from_ref(&info_albedo)).build(),
        vk::WriteDescriptorSet::builder().dst_set(dset).dst_binding(1).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(std::slice::from_ref(&info_normal)).build(),
        vk::WriteDescriptorSet::builder().dst_set(dset).dst_binding(2).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(std::slice::from_ref(&info_material)).build(),
        vk::WriteDescriptorSet::builder().dst_set(dset).dst_binding(3).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER).buffer_info(std::slice::from_ref(&lut_info)).build(),
    ];
    unsafe { ctx.device.update_descriptor_sets(&writes, &[]) };

    // Output target
    let out_format = vk::Format::R8G8B8A8_UNORM;
    let (out_img, out_mem, out_view) = create_image_2d(ctx, width, height, out_format, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC, vk::ImageAspectFlags::COLOR)?;
    let vmod2 = create_shader_module(&ctx.device, FSQ_VERT_SPV)?;
    let fmod2 = create_shader_module(&ctx.device, TOON_GBUFFER_FRAG_SPV)?;
    let stages2 = [
        vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::VERTEX).module(vmod2).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
        vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::FRAGMENT).module(fmod2).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
    ];
    let ia2 = vk::PipelineInputAssemblyStateCreateInfo::builder().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    let vp2 = vk::PipelineViewportStateCreateInfo::builder().viewport_count(1).scissor_count(1);
    let rs2 = vk::PipelineRasterizationStateCreateInfo::builder().polygon_mode(vk::PolygonMode::FILL).cull_mode(vk::CullModeFlags::NONE).front_face(vk::FrontFace::COUNTER_CLOCKWISE).line_width(1.0);
    let ms2 = vk::PipelineMultisampleStateCreateInfo::builder().rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let cb_mask = vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A;
    let cba2 = vk::PipelineColorBlendAttachmentState::builder().color_write_mask(cb_mask).blend_enable(false).build();
    let cb2 = vk::PipelineColorBlendStateCreateInfo::builder().attachments(std::slice::from_ref(&cba2));
    let ds2 = vk::PipelineDepthStencilStateCreateInfo::builder().depth_test_enable(false).depth_write_enable(false);
    let dyn_states2 = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dyn_state2 = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dyn_states2);
    let mut rendering_info2 = vk::PipelineRenderingCreateInfo::builder().color_attachment_formats(std::slice::from_ref(&out_format));
    let vi2 = vk::PipelineVertexInputStateCreateInfo::default();
    let vpci2 = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&stages2)
        .vertex_input_state(&vi2)
        .input_assembly_state(&ia2)
        .viewport_state(&vp2)
        .rasterization_state(&rs2)
        .multisample_state(&ms2)
        .depth_stencil_state(&ds2)
        .color_blend_state(&cb2)
        .dynamic_state(&dyn_state2)
        .layout(pipeline_layout)
        .push_next(&mut rendering_info2);
    let pipeline2 = unsafe { ctx.device.create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&vpci2), None) }
        .map_err(|e| anyhow!("pipeline creation failed: {:?}", e.1))?[0];

    // Record toon pass
    let pool_ci2 = vk::CommandPoolCreateInfo::builder().queue_family_index(ctx.graphics_queue_family);
    let cmd_pool2 = unsafe { ctx.device.create_command_pool(&pool_ci2, None)? };
    let alloc_ci2 = vk::CommandBufferAllocateInfo::builder().command_pool(cmd_pool2).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1);
    let cmd_buf2 = unsafe { ctx.device.allocate_command_buffers(&alloc_ci2)? }[0];
    let begin2 = vk::CommandBufferBeginInfo::builder();
    unsafe { ctx.device.begin_command_buffer(cmd_buf2, &begin2)? };
    let barrier_to_color = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .image(out_img)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
    unsafe {
        ctx.device.cmd_pipeline_barrier(
            cmd_buf2,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            std::slice::from_ref(&barrier_to_color),
        );
    }
    let clear = vk::ClearValue { color: vk::ClearColorValue { float32: [0.04, 0.04, 0.06, 1.0] } };
    let att = vk::RenderingAttachmentInfo::builder().image_view(out_view).image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::STORE).clear_value(clear);
    let render_info = vk::RenderingInfo::builder().render_area(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } }).layer_count(1).color_attachments(std::slice::from_ref(&att));
    unsafe {
        ctx.device.cmd_begin_rendering(cmd_buf2, &render_info);
        let viewport = vk::Viewport { x: 0.0, y: 0.0, width: width as f32, height: height as f32, min_depth: 0.0, max_depth: 1.0 };
        let scissor = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } };
        ctx.device.cmd_set_viewport(cmd_buf2, 0, std::slice::from_ref(&viewport));
        ctx.device.cmd_set_scissor(cmd_buf2, 0, std::slice::from_ref(&scissor));
        ctx.device.cmd_bind_pipeline(cmd_buf2, vk::PipelineBindPoint::GRAPHICS, pipeline2);
        ctx.device.cmd_bind_descriptor_sets(cmd_buf2, vk::PipelineBindPoint::GRAPHICS, pipeline_layout, 0, std::slice::from_ref(&dset), &[]);
        #[repr(C)]
        struct ToonPC { data: [f32; 12] }
        let pc = ToonPC { data: [
            0.60,   // shadowThreshold
            -1.0,   // midThreshold (disabled)
            0.20,   // rimStrength
            0.35,   // rimWidth
            0.05,   // bandSoftness
            -6.0,   // hueShiftShadowDeg (slightly cooler)
            6.0,    // hueShiftLightDeg  (slightly warmer)
            0.95,   // satScaleShadow
            1.05,   // satScaleLight
            0.86,   // specThreshold
            0.22,   // specIntensity
            0.0,    // _pad
        ]};
        let bytes = std::slice::from_raw_parts((&pc as *const ToonPC) as *const u8, std::mem::size_of::<ToonPC>());
        ctx.device.cmd_push_constants(cmd_buf2, pipeline_layout, vk::ShaderStageFlags::FRAGMENT, 0, bytes);
        ctx.device.cmd_draw(cmd_buf2, 3, 1, 0, 0);
        ctx.device.cmd_end_rendering(cmd_buf2);

        // Outline composite pass: draw backface-expanded mesh over toon using depth
        let ov = create_shader_module(&ctx.device, OUTLINE_VERT_SPV)?;
        let of = create_shader_module(&ctx.device, OUTLINE_FRAG_SPV)?;
        let stages_o = [
            vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::VERTEX).module(ov).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
            vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::FRAGMENT).module(of).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
        ];
        let binding_desc_o = vk::VertexInputBindingDescription::builder().binding(0).stride(std::mem::size_of::<crate::render::mesh::Vertex>() as u32).input_rate(vk::VertexInputRate::VERTEX).build();
        let attr_descs_o = [
            vk::VertexInputAttributeDescription::builder().location(0).binding(0).format(vk::Format::R32G32B32_SFLOAT).offset(0).build(),
            vk::VertexInputAttributeDescription::builder().location(1).binding(0).format(vk::Format::R32G32B32_SFLOAT).offset(12).build(),
        ];
        let vi_o = vk::PipelineVertexInputStateCreateInfo::builder().vertex_binding_descriptions(std::slice::from_ref(&binding_desc_o)).vertex_attribute_descriptions(&attr_descs_o);
        let ia_o = vk::PipelineInputAssemblyStateCreateInfo::builder().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let vp_o = vk::PipelineViewportStateCreateInfo::builder().viewport_count(1).scissor_count(1);
        let rs_o = vk::PipelineRasterizationStateCreateInfo::builder().polygon_mode(vk::PolygonMode::FILL).cull_mode(vk::CullModeFlags::FRONT).front_face(vk::FrontFace::COUNTER_CLOCKWISE).line_width(1.0);
        let ms_o = vk::PipelineMultisampleStateCreateInfo::builder().rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let cb_o = vk::PipelineColorBlendStateCreateInfo::builder().attachments(std::slice::from_ref(&cba2));
        let ds_o = vk::PipelineDepthStencilStateCreateInfo::builder().depth_test_enable(true).depth_write_enable(false).depth_compare_op(vk::CompareOp::LESS_OR_EQUAL);
        let dyn_states_o = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dyn_state_o = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dyn_states_o);
        let mut rendering_info_o = vk::PipelineRenderingCreateInfo::builder().color_attachment_formats(std::slice::from_ref(&out_format)).depth_attachment_format(vk::Format::D32_SFLOAT);
        // Push constant: float outline width
        let pc_range_o = vk::PushConstantRange::builder().stage_flags(vk::ShaderStageFlags::VERTEX).offset(0).size(4).build();
        let layout_ci_o = vk::PipelineLayoutCreateInfo::builder().push_constant_ranges(std::slice::from_ref(&pc_range_o));
        let pipeline_layout_o = ctx.device.create_pipeline_layout(&layout_ci_o, None)?;
        let gp_ci_o = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&stages_o)
            .vertex_input_state(&vi_o)
            .input_assembly_state(&ia_o)
            .viewport_state(&vp_o)
            .rasterization_state(&rs_o)
            .multisample_state(&ms_o)
            .depth_stencil_state(&ds_o)
            .color_blend_state(&cb_o)
            .dynamic_state(&dyn_state_o)
            .layout(pipeline_layout_o)
            .push_next(&mut rendering_info_o);
        let pipeline_o = ctx.device.create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&gp_ci_o), None)
            .map_err(|e| anyhow!("pipeline creation failed: {:?}", e.1))?[0];

        // Begin rendering over the toon target with depth
        let att_o = vk::RenderingAttachmentInfo::builder().image_view(out_view).image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::LOAD).store_op(vk::AttachmentStoreOp::STORE);
        let depth_att_o = vk::RenderingAttachmentInfo::builder().image_view(depth.2).image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::LOAD).store_op(vk::AttachmentStoreOp::STORE);
        let render_info_o = vk::RenderingInfo::builder().render_area(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } }).layer_count(1).color_attachments(std::slice::from_ref(&att_o)).depth_attachment(&depth_att_o);
        ctx.device.cmd_begin_rendering(cmd_buf2, &render_info_o);
        let viewport_o = vk::Viewport { x: 0.0, y: 0.0, width: width as f32, height: height as f32, min_depth: 0.0, max_depth: 1.0 };
        let scissor_o = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } };
        ctx.device.cmd_set_viewport(cmd_buf2, 0, std::slice::from_ref(&viewport_o));
        ctx.device.cmd_set_scissor(cmd_buf2, 0, std::slice::from_ref(&scissor_o));
        ctx.device.cmd_bind_pipeline(cmd_buf2, vk::PipelineBindPoint::GRAPHICS, pipeline_o);
        let vb_buffers_o = [vb];
        let vb_offsets_o = [0u64];
        ctx.device.cmd_bind_vertex_buffers(cmd_buf2, 0, &vb_buffers_o, &vb_offsets_o);
        ctx.device.cmd_bind_index_buffer(cmd_buf2, ib, 0, vk::IndexType::UINT32);
        let width_pc: f32 = outline_width_px
            .map(|px| px * (2.0 / width as f32))
            .unwrap_or(2.0 * (2.0 / width as f32));
        let pc_bytes = std::slice::from_raw_parts((&width_pc as *const f32) as *const u8, std::mem::size_of::<f32>());
        ctx.device.cmd_push_constants(cmd_buf2, pipeline_layout_o, vk::ShaderStageFlags::VERTEX, 0, pc_bytes);
        ctx.device.cmd_draw_indexed(cmd_buf2, inds.len() as u32, 1, 0, 0, 0);
        ctx.device.cmd_end_rendering(cmd_buf2);

        // Cleanup outline pipeline objects
        ctx.device.destroy_pipeline(pipeline_o, None);
        ctx.device.destroy_pipeline_layout(pipeline_layout_o, None);
        ctx.device.destroy_shader_module(ov, None);
        ctx.device.destroy_shader_module(of, None);
    }

    // Copy output to host
    let buf_size = (width as usize * height as usize * 4) as u64;
    let buf_ci = vk::BufferCreateInfo::builder().size(buf_size).usage(vk::BufferUsageFlags::TRANSFER_DST).sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = unsafe { ctx.device.create_buffer(&buf_ci, None)? };
    let req = unsafe { ctx.device.get_buffer_memory_requirements(buffer) };
    let mt = find_memory_type(&ctx.instance, ctx.pdevice, req.memory_type_bits, host_props)?;
    let ai = vk::MemoryAllocateInfo::builder().allocation_size(req.size).memory_type_index(mt);
    let buffer_mem = unsafe { ctx.device.allocate_memory(&ai, None)? };
    unsafe { ctx.device.bind_buffer_memory(buffer, buffer_mem, 0)? };
    let barrier_to_src = vk::ImageMemoryBarrier::builder().src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE).dst_access_mask(vk::AccessFlags::TRANSFER_READ).old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL).image(out_img).subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
    unsafe {
        ctx.device.cmd_pipeline_barrier(
            cmd_buf2,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            std::slice::from_ref(&barrier_to_src),
        );
        let region = vk::BufferImageCopy::builder().buffer_offset(0).buffer_row_length(0).buffer_image_height(0).image_subresource(vk::ImageSubresourceLayers { aspect_mask: vk::ImageAspectFlags::COLOR, mip_level: 0, base_array_layer: 0, layer_count: 1 }).image_offset(vk::Offset3D { x: 0, y: 0, z: 0 }).image_extent(vk::Extent3D { width, height, depth: 1 });
        ctx.device.cmd_copy_image_to_buffer(cmd_buf2, out_img, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, buffer, std::slice::from_ref(&region));
        ctx.device.end_command_buffer(cmd_buf2)?;
        let submit = vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&cmd_buf2));
        ctx.device.queue_submit(ctx.graphics_queue, std::slice::from_ref(&submit), vk::Fence::null())?;
        ctx.device.queue_wait_idle(ctx.graphics_queue)?;
    }

    let ptr = unsafe { ctx.device.map_memory(buffer_mem, 0, buf_size, vk::MemoryMapFlags::empty())? } as *const u8;
    let mut pixels = vec![0u8; buf_size as usize];
    unsafe { std::ptr::copy_nonoverlapping(ptr, pixels.as_mut_ptr(), pixels.len()); }
    unsafe { ctx.device.unmap_memory(buffer_mem) };

    // Cleanup
    unsafe {
        ctx.device.destroy_pipeline(pipeline2, None);
        ctx.device.destroy_pipeline_layout(pipeline_layout, None);
        ctx.device.destroy_descriptor_pool(dpool, None);
        ctx.device.destroy_descriptor_set_layout(dsl, None);
        ctx.device.destroy_sampler(sampler, None);
        ctx.device.destroy_shader_module(vmod2, None);
        ctx.device.destroy_shader_module(fmod2, None);
        ctx.device.destroy_command_pool(cmd_pool2, None);
        ctx.device.destroy_buffer(buffer, None);
        ctx.device.free_memory(buffer_mem, None);
        // LUT buffer
        ctx.device.destroy_buffer(lut_buf, None);
        ctx.device.free_memory(lut_mem, None);

        ctx.device.destroy_image_view(out_view, None);
        ctx.device.destroy_image(out_img, None);
        ctx.device.free_memory(out_mem, None);

        ctx.device.destroy_image_view(albedo.2, None);
        ctx.device.destroy_image(albedo.0, None);
        ctx.device.free_memory(albedo.1, None);
        ctx.device.destroy_image_view(normal.2, None);
        ctx.device.destroy_image(normal.0, None);
        ctx.device.free_memory(normal.1, None);
        ctx.device.destroy_image_view(material.2, None);
        ctx.device.destroy_image(material.0, None);
        ctx.device.free_memory(material.1, None);

        ctx.device.destroy_buffer(vb, None);
        ctx.device.destroy_buffer(ib, None);
        ctx.device.free_memory(vb_mem, None);
        ctx.device.free_memory(ib_mem, None);
    }

    Ok(pixels)
}

pub fn render_mesh_gbuffer_offscreen(ctx: &VkContext, width: u32, height: u32) -> Result<(Vec<u8>, Vec<u8>)> {
    use ash::vk as vk;
    use crate::render::mesh::{generate_uv_sphere, Vertex};

    // Generate a sphere that fits in clip space without projection
    let (verts, inds) = generate_uv_sphere(0.8, 32, 64);

    // Create HOST_VISIBLE vertex and index buffers
    let vb_size = (std::mem::size_of::<Vertex>() * verts.len()) as u64;
    let ib_size = (std::mem::size_of::<u32>() * inds.len()) as u64;
    let vb_ci = vk::BufferCreateInfo::builder().size(vb_size).usage(vk::BufferUsageFlags::VERTEX_BUFFER).sharing_mode(vk::SharingMode::EXCLUSIVE);
    let ib_ci = vk::BufferCreateInfo::builder().size(ib_size).usage(vk::BufferUsageFlags::INDEX_BUFFER).sharing_mode(vk::SharingMode::EXCLUSIVE);
    let vb = unsafe { ctx.device.create_buffer(&vb_ci, None)? };
    let ib = unsafe { ctx.device.create_buffer(&ib_ci, None)? };
    let vb_req = unsafe { ctx.device.get_buffer_memory_requirements(vb) };
    let ib_req = unsafe { ctx.device.get_buffer_memory_requirements(ib) };
    let host_props = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
    let vb_type = find_memory_type(&ctx.instance, ctx.pdevice, vb_req.memory_type_bits, host_props)?;
    let ib_type = find_memory_type(&ctx.instance, ctx.pdevice, ib_req.memory_type_bits, host_props)?;
    let vb_alloc = vk::MemoryAllocateInfo::builder().allocation_size(vb_req.size).memory_type_index(vb_type);
    let ib_alloc = vk::MemoryAllocateInfo::builder().allocation_size(ib_req.size).memory_type_index(ib_type);
    let vb_mem = unsafe { ctx.device.allocate_memory(&vb_alloc, None)? };
    let ib_mem = unsafe { ctx.device.allocate_memory(&ib_alloc, None)? };
    unsafe { ctx.device.bind_buffer_memory(vb, vb_mem, 0)? };
    unsafe { ctx.device.bind_buffer_memory(ib, ib_mem, 0)? };
    // Upload data
    unsafe {
        let p = ctx.device.map_memory(vb_mem, 0, vb_size, vk::MemoryMapFlags::empty())? as *mut Vertex;
        std::ptr::copy_nonoverlapping(verts.as_ptr(), p, verts.len());
        ctx.device.unmap_memory(vb_mem);
        let p = ctx.device.map_memory(ib_mem, 0, ib_size, vk::MemoryMapFlags::empty())? as *mut u32;
        std::ptr::copy_nonoverlapping(inds.as_ptr(), p, inds.len());
        ctx.device.unmap_memory(ib_mem);
    }

    // Create G-buffer attachments (color only, disable depth for simplicity)
    let albedo_format = vk::Format::R8G8B8A8_UNORM;
    let normal_format = vk::Format::R8G8B8A8_UNORM;
    let material_format = vk::Format::R8_UINT;
    let albedo = create_image_2d(ctx, width, height, albedo_format, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC, vk::ImageAspectFlags::COLOR)?;
    let normal = create_image_2d(ctx, width, height, normal_format, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC, vk::ImageAspectFlags::COLOR)?;
    let material = create_image_2d(ctx, width, height, material_format, vk::ImageUsageFlags::COLOR_ATTACHMENT, vk::ImageAspectFlags::COLOR)?;

    // Pipeline for mesh G-buffer
    let vmod = create_shader_module(&ctx.device, MESH_GBUFFER_VERT_SPV)?;
    let fmod = create_shader_module(&ctx.device, MESH_GBUFFER_FRAG_SPV)?;
    let stages = [
        vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::VERTEX).module(vmod).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
        vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::FRAGMENT).module(fmod).name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()).build(),
    ];
    // Vertex input (binding 0: pos[3], normal[3])
    let binding_desc = vk::VertexInputBindingDescription::builder().binding(0).stride(std::mem::size_of::<Vertex>() as u32).input_rate(vk::VertexInputRate::VERTEX).build();
    let attr_descs = [
        vk::VertexInputAttributeDescription::builder().location(0).binding(0).format(vk::Format::R32G32B32_SFLOAT).offset(0).build(),
        vk::VertexInputAttributeDescription::builder().location(1).binding(0).format(vk::Format::R32G32B32_SFLOAT).offset(12).build(),
    ];
    let vi = vk::PipelineVertexInputStateCreateInfo::builder().vertex_binding_descriptions(std::slice::from_ref(&binding_desc)).vertex_attribute_descriptions(&attr_descs);
    let ia = vk::PipelineInputAssemblyStateCreateInfo::builder().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    let vp = vk::PipelineViewportStateCreateInfo::builder().viewport_count(1).scissor_count(1);
    let rs = vk::PipelineRasterizationStateCreateInfo::builder().polygon_mode(vk::PolygonMode::FILL).cull_mode(vk::CullModeFlags::BACK).front_face(vk::FrontFace::COUNTER_CLOCKWISE).line_width(1.0);
    let ms = vk::PipelineMultisampleStateCreateInfo::builder().rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let cb_mask = vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A;
    let cba = [
        vk::PipelineColorBlendAttachmentState::builder().color_write_mask(cb_mask).blend_enable(false).build(),
        vk::PipelineColorBlendAttachmentState::builder().color_write_mask(cb_mask).blend_enable(false).build(),
        vk::PipelineColorBlendAttachmentState::builder().color_write_mask(cb_mask).blend_enable(false).build(),
    ];
    let cb = vk::PipelineColorBlendStateCreateInfo::builder().attachments(&cba);
    let ds = vk::PipelineDepthStencilStateCreateInfo::builder().depth_test_enable(false).depth_write_enable(false);
    let dyn_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dyn_state = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dyn_states);
    let color_formats = [albedo_format, normal_format, material_format];
    let mut rendering_info = vk::PipelineRenderingCreateInfo::builder().color_attachment_formats(&color_formats);
    let layout_ci = vk::PipelineLayoutCreateInfo::builder();
    let pipeline_layout = unsafe { ctx.device.create_pipeline_layout(&layout_ci, None)? };
    let gp_ci = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&stages)
        .vertex_input_state(&vi)
        .input_assembly_state(&ia)
        .viewport_state(&vp)
        .rasterization_state(&rs)
        .multisample_state(&ms)
        .depth_stencil_state(&ds)
        .color_blend_state(&cb)
        .dynamic_state(&dyn_state)
        .layout(pipeline_layout)
        .push_next(&mut rendering_info);
    let pipeline = unsafe { ctx.device.create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&gp_ci), None) }
        .map_err(|e| anyhow!("pipeline creation failed: {:?}", e.1))?[0];

    // Command buffer
    let pool_ci = vk::CommandPoolCreateInfo::builder().queue_family_index(ctx.graphics_queue_family);
    let cmd_pool = unsafe { ctx.device.create_command_pool(&pool_ci, None)? };
    let alloc_ci = vk::CommandBufferAllocateInfo::builder().command_pool(cmd_pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1);
    let cmd_buf = unsafe { ctx.device.allocate_command_buffers(&alloc_ci)? }[0];
    let begin = vk::CommandBufferBeginInfo::builder();
    unsafe { ctx.device.begin_command_buffer(cmd_buf, &begin)? };

    // Transitions
    let to_color = |image| vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
        .build();
    let barriers = [to_color(albedo.0), to_color(normal.0), to_color(material.0)];
    unsafe {
        ctx.device.cmd_pipeline_barrier(
            cmd_buf,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &barriers,
        );
    }

    // Begin rendering
    let clear0 = vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } };
    let clear1 = vk::ClearValue { color: vk::ClearColorValue { float32: [0.5, 0.5, 1.0, 1.0] } };
    let att0 = vk::RenderingAttachmentInfo::builder().image_view(albedo.2).image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::STORE).clear_value(clear0).build();
    let att1 = vk::RenderingAttachmentInfo::builder().image_view(normal.2).image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::STORE).clear_value(clear1).build();
    let clear_mat = vk::ClearValue { color: vk::ClearColorValue { uint32: [2, 0, 0, 0] } };
    let att2 = vk::RenderingAttachmentInfo::builder().image_view(material.2).image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::STORE).clear_value(clear_mat).build();
    let color_atts = [att0, att1, att2];
    let render_info = vk::RenderingInfo::builder()
        .render_area(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } })
        .layer_count(1)
        .color_attachments(&color_atts);
    unsafe {
        ctx.device.cmd_begin_rendering(cmd_buf, &render_info);
        let viewport = vk::Viewport { x: 0.0, y: 0.0, width: width as f32, height: height as f32, min_depth: 0.0, max_depth: 1.0 };
        let scissor = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } };
        ctx.device.cmd_set_viewport(cmd_buf, 0, std::slice::from_ref(&viewport));
        ctx.device.cmd_set_scissor(cmd_buf, 0, std::slice::from_ref(&scissor));
        ctx.device.cmd_bind_pipeline(cmd_buf, vk::PipelineBindPoint::GRAPHICS, pipeline);
        let vb_buffers = [vb];
        let vb_offsets = [0u64];
        ctx.device.cmd_bind_vertex_buffers(cmd_buf, 0, &vb_buffers, &vb_offsets);
        ctx.device.cmd_bind_index_buffer(cmd_buf, ib, 0, vk::IndexType::UINT32);
        ctx.device.cmd_draw_indexed(cmd_buf, inds.len() as u32, 1, 0, 0, 0);
        ctx.device.cmd_end_rendering(cmd_buf);
    }

    // Transition to TRANSFER_SRC and copy to CPU
    let to_src = |image| vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
        .build();
    let barriers2 = [to_src(albedo.0), to_src(normal.0)];
    unsafe {
        ctx.device.cmd_pipeline_barrier(
            cmd_buf,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &barriers2,
        );
    }

    let buf_size = (width as usize * height as usize * 4) as u64;
    let make_buffer = |usage: vk::BufferUsageFlags| -> Result<(vk::Buffer, vk::DeviceMemory)> {
        let ci = vk::BufferCreateInfo::builder().size(buf_size).usage(usage).sharing_mode(vk::SharingMode::EXCLUSIVE);
        let b = unsafe { ctx.device.create_buffer(&ci, None)? };
        let req = unsafe { ctx.device.get_buffer_memory_requirements(b) };
        let mt = find_memory_type(&ctx.instance, ctx.pdevice, req.memory_type_bits, host_props)?;
        let ai = vk::MemoryAllocateInfo::builder().allocation_size(req.size).memory_type_index(mt);
        let mem = unsafe { ctx.device.allocate_memory(&ai, None)? };
        unsafe { ctx.device.bind_buffer_memory(b, mem, 0)? };
        Ok((b, mem))
    };
    let (buf_a, mem_a) = make_buffer(vk::BufferUsageFlags::TRANSFER_DST)?;
    let (buf_n, mem_n) = make_buffer(vk::BufferUsageFlags::TRANSFER_DST)?;
    let region = vk::BufferImageCopy::builder()
        .buffer_offset(0)
        .buffer_row_length(0)
        .buffer_image_height(0)
        .image_subresource(vk::ImageSubresourceLayers { aspect_mask: vk::ImageAspectFlags::COLOR, mip_level: 0, base_array_layer: 0, layer_count: 1 })
        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .image_extent(vk::Extent3D { width, height, depth: 1 });
    unsafe {
        ctx.device.cmd_copy_image_to_buffer(cmd_buf, albedo.0, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, buf_a, std::slice::from_ref(&region));
        ctx.device.cmd_copy_image_to_buffer(cmd_buf, normal.0, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, buf_n, std::slice::from_ref(&region));
        ctx.device.end_command_buffer(cmd_buf)?;
        let submit = vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&cmd_buf));
        ctx.device.queue_submit(ctx.graphics_queue, std::slice::from_ref(&submit), vk::Fence::null())?;
        ctx.device.queue_wait_idle(ctx.graphics_queue)?;
    }

    let read_back = |mem: vk::DeviceMemory| -> Result<Vec<u8>> {
        let ptr = unsafe { ctx.device.map_memory(mem, 0, buf_size, vk::MemoryMapFlags::empty())? } as *const u8;
        let mut v = vec![0u8; buf_size as usize];
        unsafe { std::ptr::copy_nonoverlapping(ptr, v.as_mut_ptr(), v.len()); }
        unsafe { ctx.device.unmap_memory(mem) };
        Ok(v)
    };
    let albedo_pixels = read_back(mem_a)?;
    let normal_pixels = read_back(mem_n)?;

    // Cleanup
    unsafe {
        ctx.device.destroy_pipeline(pipeline, None);
        ctx.device.destroy_pipeline_layout(pipeline_layout, None);
        ctx.device.destroy_shader_module(vmod, None);
        ctx.device.destroy_shader_module(fmod, None);
        ctx.device.destroy_command_pool(cmd_pool, None);
        ctx.device.destroy_buffer(buf_a, None);
        ctx.device.destroy_buffer(buf_n, None);
        ctx.device.free_memory(mem_a, None);
        ctx.device.free_memory(mem_n, None);
        ctx.device.destroy_image_view(material.2, None);
        ctx.device.destroy_image(material.0, None);
        ctx.device.free_memory(material.1, None);
        ctx.device.destroy_image_view(albedo.2, None);
        ctx.device.destroy_image(albedo.0, None);
        ctx.device.free_memory(albedo.1, None);
        ctx.device.destroy_image_view(normal.2, None);
        ctx.device.destroy_image(normal.0, None);
        ctx.device.free_memory(normal.1, None);
        ctx.device.destroy_buffer(vb, None);
        ctx.device.destroy_buffer(ib, None);
        ctx.device.free_memory(vb_mem, None);
        ctx.device.free_memory(ib_mem, None);
    }

    Ok((albedo_pixels, normal_pixels))
}
