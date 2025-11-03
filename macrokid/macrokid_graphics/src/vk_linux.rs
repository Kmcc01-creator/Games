//! Linux-facing Vulkan app scaffold using `ash` + `winit`.
//!
//! This module is gated behind the `vulkan-linux` feature to avoid pulling
//! heavy graphics deps by default. It focuses on minimal instance/device
//! bring-up and an event loop. Swapchain and rendering are intentionally
//! deferred to keep the first integration small and reviewable.
#![cfg(feature = "vulkan-linux")]

use crate::engine::EngineConfig;
use crate::render_graph::{PassDesc, plan_resources_from_passes};
use crate::resources::{ResourceBindings, VertexLayout, StepMode};

use ash::{vk, Entry, Instance};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::ffi::{CString, CStr};

#[derive(Debug)]
pub enum VkError {
    General(String),
}

impl From<&str> for VkError { fn from(s: &str) -> Self { VkError::General(s.into()) } }
impl From<String> for VkError { fn from(s: String) -> Self { VkError::General(s) } }

/// Optional app-supplied resources for demo purposes.
#[derive(Debug, Default)]
pub struct AppResources {
    pub uniform_data: Option<Vec<u8>>,      // initial data copied into each frame's 64-byte uniform buffer
    pub image_rgba: Option<[u8; 4]>,        // solid color written to a 1x1 image if provided
    pub image_size: Option<(u32, u32)>,     // size for image_pixels
    pub image_pixels: Option<Vec<u8>>,      // RGBA8 pixels; len == w*h*4 when image_size is Some
}

/// Thread-local command pool infrastructure for parallel command buffer recording.
///
/// Each thread gets its own command pool (Vulkan requirement), allowing secondary
/// command buffers to be recorded in parallel without contention.
struct ThreadLocalPools {
    device: ash::Device,
    pools: Vec<vk::CommandPool>,                             // One pool per thread
    secondary_buffers: Vec<Vec<vk::CommandBuffer>>,         // [thread_idx][buffer_idx]
    queue_family_index: u32,
    max_secondary_per_thread: u32,
}

impl ThreadLocalPools {
    /// Create thread-local pools for a given number of worker threads.
    unsafe fn new(
        device: &ash::Device,
        queue_family_index: u32,
        num_threads: usize,
        max_secondary_per_thread: u32,
    ) -> Result<Self, VkError> {
        let mut pools = Vec::with_capacity(num_threads);
        let mut secondary_buffers = Vec::with_capacity(num_threads);

        for _ in 0..num_threads {
            // Create command pool with RESET_COMMAND_BUFFER flag (can reuse buffers)
            let pool_info = vk::CommandPoolCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

            let pool = device
                .create_command_pool(&pool_info, None)
                .map_err(|e| VkError::General(format!("create_command_pool(thread_local): {e}")))?;

            // Allocate secondary command buffers from this pool
            let alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(pool)
                .level(vk::CommandBufferLevel::SECONDARY)
                .command_buffer_count(max_secondary_per_thread);

            let buffers = device
                .allocate_command_buffers(&alloc_info)
                .map_err(|e| VkError::General(format!("allocate_command_buffers(secondary): {e}")))?;

            pools.push(pool);
            secondary_buffers.push(buffers);
        }

        Ok(Self {
            device: device.clone(),
            pools,
            secondary_buffers,
            queue_family_index,
            max_secondary_per_thread,
        })
    }

    /// Get a secondary command buffer for a specific thread.
    ///
    /// # Safety
    /// - `thread_idx` must be < num_threads
    /// - `buffer_idx` must be < max_secondary_per_thread
    /// - Only one thread should access buffers from its own pool
    unsafe fn get_secondary(&self, thread_idx: usize, buffer_idx: usize) -> vk::CommandBuffer {
        self.secondary_buffers[thread_idx][buffer_idx]
    }

    /// Reset a thread's command pool (all buffers in that pool).
    unsafe fn reset_pool(&self, thread_idx: usize) -> Result<(), VkError> {
        self.device
            .reset_command_pool(
                self.pools[thread_idx],
                vk::CommandPoolResetFlags::empty(),
            )
            .map_err(|e| VkError::General(format!("reset_command_pool(thread {}): {e}", thread_idx)))
    }

    /// Get the number of threads this pool set supports.
    fn num_threads(&self) -> usize {
        self.pools.len()
    }

    /// Cleanup all pools and buffers.
    unsafe fn destroy(&mut self) {
        for pool in &self.pools {
            self.device.destroy_command_pool(*pool, None);
        }
        self.pools.clear();
        self.secondary_buffers.clear();
    }
}

struct VkCore {
    _entry: Entry,
    instance: Instance,
    surface_loader: ash::extensions::khr::Surface,
    surface: vk::SurfaceKHR,
    phys: vk::PhysicalDevice,
    device: ash::Device,
    queue: vk::Queue,
    queue_family_index: u32,
    // Swapchain + views
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    surface_format: vk::SurfaceFormatKHR,
    extent: vk::Extent2D,
    present_mode: vk::PresentModeKHR,
    images: Vec<vk::Image>,
    views: Vec<vk::ImageView>,
    // Render pass + framebuffers
    render_pass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
    // Optional MRT offscreen targets (one per declared color target; each has per-frame image+view)
    mrt_formats: Vec<vk::Format>,
    mrt_images: Vec<Vec<vk::Image>>,         // [target][frame]
    mrt_memories: Vec<Vec<vk::DeviceMemory>>,// [target][frame]
    mrt_views: Vec<Vec<vk::ImageView>>,      // [target][frame]
    // Depth resources
    depth_format: vk::Format,
    depth_image: vk::Image,
    depth_memory: vk::DeviceMemory,
    depth_view: vk::ImageView,
    // Stub pipeline layout used for now
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    compute_pipelines: Vec<vk::Pipeline>,
    compute_dispatches: Vec<(u32, u32, u32)>,
    set_layouts: Vec<vk::DescriptorSetLayout>, // graphics/global
    compute_set_layouts: Vec<Vec<vk::DescriptorSetLayout>>, // per-compute
    compute_pipeline_layouts: Vec<vk::PipelineLayout>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets_per_frame: Vec<Vec<vk::DescriptorSet>>,
    compute_descriptor_sets_per_frame: Vec<Vec<Vec<vk::DescriptorSet>>>,
    // Minimal demo resources for descriptor writes
    uniform_buffers: Vec<vk::Buffer>,
    uniform_memories: Vec<vk::DeviceMemory>,
    demo_sampler: vk::Sampler,
    demo_image: vk::Image,
    demo_image_memory: vk::DeviceMemory,
    demo_image_view: vk::ImageView,
    demo_storage_buffers: Vec<vk::Buffer>,
    demo_storage_memories: Vec<vk::DeviceMemory>,
    demo_storage_images: Vec<vk::Image>,
    demo_storage_image_memories: Vec<vk::DeviceMemory>,
    demo_storage_image_views: Vec<vk::ImageView>,
    dyn_viewport: bool,
    dyn_scissor: bool,
    // Size of per-frame uniform buffer in bytes (0 when no uniform binding)
    uniform_size_bytes: u64,
    // Command recording + sync
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available: Vec<vk::Semaphore>,
    render_finished: Vec<vk::Semaphore>,
    in_flight: Vec<vk::Fence>,
    // Dummy vertex buffer (to demonstrate binding)
    vertex_buffer: vk::Buffer,
    vertex_memory: vk::DeviceMemory,
    // Multi-threaded command buffer recording (optional)
    thread_pools: Option<ThreadLocalPools>,
}

impl VkCore {
    fn new_with<RB, VL>(window: &winit::window::Window, cfg: &EngineConfig, resources: Option<&AppResources>) -> Result<Self, VkError>
    where
        RB: ResourceBindings,
        VL: VertexLayout,
    {
        unsafe {
            // 1) Entry + Instance
            let entry = Entry::linked();
            let app_name = CString::new("macrokid_vulkan_linux").unwrap();
            let app_info = vk::ApplicationInfo::builder()
                .application_name(&app_name)
                .application_version(vk::make_api_version(0, 0, 1, 0))
                .engine_name(&app_name)
                .engine_version(vk::make_api_version(0, 0, 1, 0))
                .api_version(vk::API_VERSION_1_2);

            let display_handle = window.raw_display_handle();
            let window_handle = window.raw_window_handle();
            let mut ext_names = ash_window::enumerate_required_extensions(display_handle)
                .map_err(|e| VkError::General(format!("enumerate_required_extensions: {e}")))?
                .to_vec();

            // Surface + platform extensions are included via ash-window helper above.
            let create_info = vk::InstanceCreateInfo::builder().application_info(&app_info).enabled_extension_names(&ext_names);
            let instance = entry.create_instance(&create_info, None)
                .map_err(|e| VkError::General(format!("create_instance: {e}")))?;

            // 2) Surface
            let surface = ash_window::create_surface(&entry, &instance, display_handle, window_handle, None)
                .map_err(|e| VkError::General(format!("create_surface: {e}")))?;
            let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);

            // 3) Physical device + queue family supporting graphics + present
            let phys_devices = instance.enumerate_physical_devices()
                .map_err(|e| VkError::General(format!("enumerate_physical_devices: {e}")))?;

            // Build ordered candidates honoring adapter_index or adapter_preference
            let mut ordered: Vec<vk::PhysicalDevice> = Vec::new();
            if let Some(idx) = cfg.options.adapter_index {
                if idx < phys_devices.len() { ordered.push(phys_devices[idx]); }
            }
            if ordered.is_empty() {
                if let Some(pref) = cfg.options.adapter_preference.as_deref() {
                    let want = match pref.to_ascii_lowercase().as_str() {
                        "discrete" => Some(vk::PhysicalDeviceType::DISCRETE_GPU),
                        "integrated" => Some(vk::PhysicalDeviceType::INTEGRATED_GPU),
                        "virtual" => Some(vk::PhysicalDeviceType::VIRTUAL_GPU),
                        "cpu" => Some(vk::PhysicalDeviceType::CPU),
                        _ => None,
                    };
                    if let Some(want_ty) = want {
                        for &pd in &phys_devices {
                            let props = instance.get_physical_device_properties(pd);
                            if props.device_type == want_ty { ordered.push(pd); }
                        }
                        for &pd in &phys_devices { if !ordered.contains(&pd) { ordered.push(pd); } }
                    }
                }
            }
            if ordered.is_empty() { ordered = phys_devices.clone(); }

            let (phys, qfi) = {
                let mut chosen: Option<(vk::PhysicalDevice, u32)> = None;
                'outer: for &pd in &ordered {
                    let qfams = instance.get_physical_device_queue_family_properties(pd);
                    for (i, qf) in qfams.iter().enumerate() {
                        let i_u32 = i as u32;
                        let supports_graphics = qf.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                        let present_ok = surface_loader.get_physical_device_surface_support(pd, i_u32, surface).unwrap_or(false);
                        if supports_graphics && present_ok { chosen = Some((pd, i_u32)); break 'outer; }
                    }
                }
                chosen.ok_or_else(|| VkError::General("no suitable queue family with graphics+present".into()))?
            };

            // 4) Logical device + queue (+ swapchain extension)
            let priorities = [1.0f32];
            let qci = [vk::DeviceQueueCreateInfo::builder().queue_family_index(qfi).queue_priorities(&priorities).build()];
            let device_exts = [ash::extensions::khr::Swapchain::name().as_ptr()];
            let device_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(&qci)
                .enabled_extension_names(&device_exts);
            let device = instance.create_device(phys, &device_info, None)
                .map_err(|e| VkError::General(format!("create_device: {e}")))?;
            let queue = device.get_device_queue(qfi, 0);

            // 5) Swapchain select + create
            let surface_caps = surface_loader
                .get_physical_device_surface_capabilities(phys, surface)
                .map_err(|e| VkError::General(format!("surface_capabilities: {e}")))?;
            let surface_formats = surface_loader
                .get_physical_device_surface_formats(phys, surface)
                .map_err(|e| VkError::General(format!("surface_formats: {e}")))?;
            // Select color format/colorspace, honoring cfg.options when provided
            let map_format = |s: &str| -> Option<vk::Format> {
                match s {
                    "B8G8R8A8_SRGB" => Some(vk::Format::B8G8R8A8_SRGB),
                    "R8G8B8A8_UNORM" => Some(vk::Format::R8G8B8A8_UNORM),
                    _ => None,
                }
            };
            let map_colorspace = |s: &str| -> Option<vk::ColorSpaceKHR> {
                match s {
                    "SRGB_NONLINEAR" => Some(vk::ColorSpaceKHR::SRGB_NONLINEAR),
                    _ => None,
                }
            };
            let requested_fmt = cfg.options.color_format.and_then(map_format);
            let requested_cs = cfg.options.color_space.and_then(map_colorspace);
            let surface_format = surface_formats
                .iter()
                .cloned()
                .find(|f| {
                    let fmt_ok = requested_fmt.map(|want| f.format == want).unwrap_or(true);
                    let cs_ok = requested_cs.map(|want| f.color_space == want).unwrap_or(true);
                    fmt_ok && cs_ok
                })
                .or_else(|| surface_formats
                    .iter()
                    .cloned()
                    .find(|f| f.format == vk::Format::B8G8R8A8_SRGB && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR))
                .or_else(|| surface_formats.first().cloned())
                .ok_or_else(|| VkError::General("no surface formats".into()))?;

            let present_modes = surface_loader
                .get_physical_device_surface_present_modes(phys, surface)
                .map_err(|e| VkError::General(format!("present_modes: {e}")))?;
            // Determine desired present mode: priority list > explicit option > vsync heuristic
            let map_present = |s: &str| -> Option<vk::PresentModeKHR> {
                match s.to_ascii_uppercase().as_str() {
                    "MAILBOX" => Some(vk::PresentModeKHR::MAILBOX),
                    "FIFO" => Some(vk::PresentModeKHR::FIFO),
                    "IMMEDIATE" => Some(vk::PresentModeKHR::IMMEDIATE),
                    "FIFO_RELAXED" => Some(vk::PresentModeKHR::FIFO_RELAXED),
                    _ => None,
                }
            };
            let mut present_mode: vk::PresentModeKHR = vk::PresentModeKHR::FIFO; // safe default
            if let Some(list) = &cfg.options.present_mode_priority {
                for &name in list.iter() {
                    if let Some(pm) = map_present(name) {
                        if present_modes.iter().any(|m| *m == pm) { present_mode = pm; break; }
                    }
                }
                if present_mode == vk::PresentModeKHR::FIFO && !cfg.window.vsync {
                    // try common low-latency default if not explicitly found
                    if present_modes.iter().any(|m| *m == vk::PresentModeKHR::MAILBOX) { present_mode = vk::PresentModeKHR::MAILBOX; }
                }
            } else {
                let desired_present = cfg.options.present_mode.and_then(map_present).or_else(|| {
                    if cfg.window.vsync { Some(vk::PresentModeKHR::FIFO) } else { Some(vk::PresentModeKHR::MAILBOX) }
                }).unwrap();
                present_mode = present_modes.iter().cloned().find(|m| *m == desired_present)
                    .or_else(|| present_modes.iter().cloned().find(|m| *m == vk::PresentModeKHR::MAILBOX))
                    .unwrap_or(vk::PresentModeKHR::FIFO);
            }

            // Log chosen adapter and present mode
            let props = instance.get_physical_device_properties(phys);
            let name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) }.to_string_lossy();
            let present_mode_name = match present_mode {
                vk::PresentModeKHR::IMMEDIATE => "IMMEDIATE",
                vk::PresentModeKHR::MAILBOX => "MAILBOX",
                vk::PresentModeKHR::FIFO => "FIFO",
                vk::PresentModeKHR::FIFO_RELAXED => "FIFO_RELAXED",
                vk::PresentModeKHR::SHARED_DEMAND_REFRESH => "SHARED_DEMAND_REFRESH",
                vk::PresentModeKHR::SHARED_CONTINUOUS_REFRESH => "SHARED_CONTINUOUS_REFRESH",
                _ => "UNKNOWN",
            };
            let device_type_name = match props.device_type {
                vk::PhysicalDeviceType::INTEGRATED_GPU => "IntegratedGPU",
                vk::PhysicalDeviceType::DISCRETE_GPU => "DiscreteGPU",
                vk::PhysicalDeviceType::VIRTUAL_GPU => "VirtualGPU",
                vk::PhysicalDeviceType::CPU => "CPU",
                _ => "Other",
            };
            println!("[vk-linux] adapter='{}' type={} present_mode={}", name, device_type_name, present_mode_name);

            let win_size = window.inner_size();
            let extent = if surface_caps.current_extent.width != u32::MAX {
                surface_caps.current_extent
            } else {
                vk::Extent2D {
                    width: win_size.width.clamp(surface_caps.min_image_extent.width, surface_caps.max_image_extent.width),
                    height: win_size.height.clamp(surface_caps.min_image_extent.height, surface_caps.max_image_extent.height),
                }
            };

            let mut image_count = surface_caps.min_image_count + 1;
            if let Some(req) = cfg.options.swapchain_images { image_count = req.max(surface_caps.min_image_count); }
            if surface_caps.max_image_count > 0 {
                image_count = image_count.min(surface_caps.max_image_count);
            }

            let (image_sharing_mode, queue_family_indices) = (vk::SharingMode::EXCLUSIVE, Vec::<u32>::new());
            let swapchain_info = vk::SwapchainCreateInfoKHR::builder()
                .surface(surface)
                .min_image_count(image_count)
                .image_color_space(surface_format.color_space)
                .image_format(surface_format.format)
                .image_extent(extent)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(image_sharing_mode)
                .queue_family_indices(&queue_family_indices)
                .pre_transform(surface_caps.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
                .image_array_layers(1);

            let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &device);
            let swapchain = swapchain_loader
                .create_swapchain(&swapchain_info, None)
                .map_err(|e| VkError::General(format!("create_swapchain: {e}")))?;
            let images = swapchain_loader
                .get_swapchain_images(swapchain)
                .map_err(|e| VkError::General(format!("get_swapchain_images: {e}")))?;

            // 6) Image views
            let mut views = Vec::with_capacity(images.len());
            for &image in &images {
                let subresource = vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                };
                let iv_info = vk::ImageViewCreateInfo::builder()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(surface_format.format)
                    .subresource_range(subresource);
                let view = device
                    .create_image_view(&iv_info, None)
                    .map_err(|e| VkError::General(format!("create_image_view: {e}")))?;
                views.push(view);
            }

            // 7) Render pass (color + optional depth)
            // Resolve sample count for attachments from first pipeline or override via options
            let samples_from_opt = |n: u32| -> vk::SampleCountFlags { match n { 1 => vk::SampleCountFlags::TYPE_1, 2 => vk::SampleCountFlags::TYPE_2, 4 => vk::SampleCountFlags::TYPE_4, 8 => vk::SampleCountFlags::TYPE_8, 16 => vk::SampleCountFlags::TYPE_16, 32 => vk::SampleCountFlags::TYPE_32, 64 => vk::SampleCountFlags::TYPE_64, _ => vk::SampleCountFlags::TYPE_1 } };
            let first_desc = cfg.pipelines.first().ok_or_else(|| VkError::General("no pipelines in EngineConfig".into()))?;
            let default_samples = crate::vk_bridge::samples_from(first_desc);
            let rp_samples = cfg.options.msaa_samples.map(samples_from_opt).unwrap_or(default_samples);

            // Compute-only present path: if enabled and compute pipelines exist, we will not use a graphics render pass
            let compute_only_present = cfg.options.compute_only_present.unwrap_or(false) && !cfg.compute_pipelines.is_empty();
            let mut use_mrt = first_desc.color_targets.map(|s| !s.is_empty()).unwrap_or(false);
            if compute_only_present && !use_mrt { use_mrt = true; }
            let mut mrt_formats: Vec<vk::Format> = Vec::new();
            let mut color_attachments: Vec<vk::AttachmentDescription> = Vec::new();
            if use_mrt {
                if let Some(cts) = first_desc.color_targets {
                    for ct in cts {
                        let fmt = crate::vk_bridge::parse_color_format(ct.format).unwrap_or(vk::Format::R16G16B16A16_SFLOAT);
                        mrt_formats.push(fmt);
                        color_attachments.push(
                            vk::AttachmentDescription::builder()
                                .format(fmt)
                                .samples(rp_samples)
                                .load_op(vk::AttachmentLoadOp::CLEAR)
                                .store_op(vk::AttachmentStoreOp::STORE)
                                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                                .initial_layout(vk::ImageLayout::UNDEFINED)
                                .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                                .build()
                        );
                    }
                } else {
                    // Compute-only path with implicit MRT: choose a sensible HDR default
                    let fmt = vk::Format::R16G16B16A16_SFLOAT;
                    mrt_formats.push(fmt);
                    // No color attachments needed for render pass when compute-only; skip adding to color_attachments
                }
            } else {
                color_attachments.push(
                    vk::AttachmentDescription::builder()
                        .format(surface_format.format)
                        .samples(rp_samples)
                        .load_op(vk::AttachmentLoadOp::CLEAR)
                        .store_op(vk::AttachmentStoreOp::STORE)
                        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                        .initial_layout(vk::ImageLayout::UNDEFINED)
                        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                        .build()
                );
            }
            // Choose a depth format
            let pick_depth_format = |candidates: &[vk::Format]| -> Option<vk::Format> {
                for &fmt in candidates { 
                    let props = instance.get_physical_device_format_properties(phys, fmt);
                    if props.optimal_tiling_features.contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT) { return Some(fmt); }
                }
                None
            };
            let requested_depth = cfg.options.depth_format.and_then(crate::vk_bridge::parse_depth_format);
            let depth_format = if let Some(df) = requested_depth {
                pick_depth_format(&[df]).unwrap_or(df)
            } else {
                pick_depth_format(&[
                    vk::Format::D32_SFLOAT,
                    vk::Format::D32_SFLOAT_S8_UINT,
                    vk::Format::D24_UNORM_S8_UINT,
                ]).unwrap_or(vk::Format::D32_SFLOAT)
            };
            let depth_attachment = vk::AttachmentDescription::builder()
                .format(depth_format)
                .samples(rp_samples)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .build();
            let mut atts: Vec<vk::AttachmentDescription> = Vec::new();
            atts.extend(color_attachments.iter().cloned());
            atts.push(depth_attachment);
            let mut color_refs: Vec<vk::AttachmentReference> = Vec::new();
            for i in 0..color_attachments.len() { color_refs.push(vk::AttachmentReference { attachment: i as u32, layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL }); }
            let depth_attachment_ref = vk::AttachmentReference { attachment: color_attachments.len() as u32, layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL };
            let subpass = vk::SubpassDescription::builder()
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(&color_refs)
                .depth_stencil_attachment(&depth_attachment_ref)
                .build();
            let dependency = vk::SubpassDependency::builder()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
                .build();
            let rp_info = vk::RenderPassCreateInfo::builder()
                .attachments(&atts)
                .subpasses(std::slice::from_ref(&subpass))
                .dependencies(std::slice::from_ref(&dependency));
            let render_pass = device
                .create_render_pass(&rp_info, None)
                .map_err(|e| VkError::General(format!("create_render_pass: {e}")))?;

            // 8) Depth image/resources
            let depth_image_info = vk::ImageCreateInfo::builder()
                .image_type(vk::ImageType::TYPE_2D)
                .format(depth_format)
                .extent(vk::Extent3D { width: extent.width, height: extent.height, depth: 1 })
                .mip_levels(1)
                .array_layers(1)
                .samples(rp_samples)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED);
            let depth_image = device.create_image(&depth_image_info, None).map_err(|e| VkError::General(format!("create_image(depth): {e}")))?;
            let depth_req = device.get_image_memory_requirements(depth_image);
            let mem_props = instance.get_physical_device_memory_properties(phys);
            let find_type = |type_bits: u32, flags: vk::MemoryPropertyFlags| -> Option<u32> {
                for i in 0..mem_props.memory_type_count { if (type_bits & (1 << i)) != 0 && mem_props.memory_types[i as usize].property_flags.contains(flags) { return Some(i); } }
                None
            };
            let depth_type = find_type(depth_req.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL)
                .or_else(|| find_type(depth_req.memory_type_bits, vk::MemoryPropertyFlags::empty()))
                .ok_or_else(|| VkError::General("no memory type for depth".into()))?;
            let depth_alloc = vk::MemoryAllocateInfo::builder().allocation_size(depth_req.size).memory_type_index(depth_type);
            let depth_memory = device.allocate_memory(&depth_alloc, None).map_err(|e| VkError::General(format!("allocate_memory(depth): {e}")))?;
            device.bind_image_memory(depth_image, depth_memory, 0).map_err(|e| VkError::General(format!("bind_image_memory(depth): {e}")))?;
            let depth_aspect = if depth_format == vk::Format::D32_SFLOAT || depth_format == vk::Format::D16_UNORM || depth_format == vk::Format::D32_SFLOAT_S8_UINT || depth_format == vk::Format::D24_UNORM_S8_UINT { vk::ImageAspectFlags::DEPTH } else { vk::ImageAspectFlags::DEPTH };
            let depth_view_info = vk::ImageViewCreateInfo::builder()
                .image(depth_image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(depth_format)
                .subresource_range(vk::ImageSubresourceRange { aspect_mask: depth_aspect, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
            let depth_view = device.create_image_view(&depth_view_info, None).map_err(|e| VkError::General(format!("create_image_view(depth): {e}")))?;

            // 9) Framebuffers and optional MRT offscreen color images
            let mut mrt_images: Vec<Vec<vk::Image>> = Vec::new();
            let mut mrt_memories: Vec<Vec<vk::DeviceMemory>> = Vec::new();
            let mut mrt_views: Vec<Vec<vk::ImageView>> = Vec::new();
            if use_mrt {
                for &fmt in &mrt_formats {
                    let mut imgs = Vec::new();
                    let mut mems = Vec::new();
                    let mut ivs = Vec::new();
                    for _ in 0..views.len() {
                        let mut img_info = vk::ImageCreateInfo::builder()
                            .image_type(vk::ImageType::TYPE_2D)
                            .format(fmt)
                            .extent(vk::Extent3D { width: extent.width, height: extent.height, depth: 1 })
                            .mip_levels(1)
                            .array_layers(1)
                            .samples(rp_samples)
                            .tiling(vk::ImageTiling::OPTIMAL)
                            .usage(if compute_only_present { vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC } else { vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC })
                            .sharing_mode(vk::SharingMode::EXCLUSIVE)
                            .initial_layout(vk::ImageLayout::UNDEFINED);
                        let image = device.create_image(&img_info, None).map_err(|e| VkError::General(format!("create_image(mrt): {e}")))?;
                        let req = device.get_image_memory_requirements(image);
                        let ty = find_type(req.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL)
                            .or_else(|| find_type(req.memory_type_bits, vk::MemoryPropertyFlags::empty()))
                            .ok_or_else(|| VkError::General("no memory type for mrt color".into()))?;
                        let alloc = vk::MemoryAllocateInfo::builder().allocation_size(req.size).memory_type_index(ty);
                        let mem = device.allocate_memory(&alloc, None).map_err(|e| VkError::General(format!("allocate_memory(mrt): {e}")))?;
                        device.bind_image_memory(image, mem, 0).map_err(|e| VkError::General(format!("bind_image_memory(mrt): {e}")))?;
                        let iv_info = vk::ImageViewCreateInfo::builder()
                            .image(image)
                            .view_type(vk::ImageViewType::TYPE_2D)
                            .format(fmt)
                            .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
                        let view = device.create_image_view(&iv_info, None).map_err(|e| VkError::General(format!("create_image_view(mrt): {e}")))?;
                        imgs.push(image); mems.push(mem); ivs.push(view);
                    }
                    mrt_images.push(imgs);
                    mrt_memories.push(mems);
                    mrt_views.push(ivs);
                }
            }

            let mut framebuffers = Vec::with_capacity(views.len());
            if !compute_only_present {
                for fi in 0..views.len() {
                    if use_mrt {
                        let mut atts: Vec<vk::ImageView> = Vec::new();
                        for t in 0..mrt_views.len() { atts.push(mrt_views[t][fi]); }
                        atts.push(depth_view);
                        let fb_info = vk::FramebufferCreateInfo::builder()
                            .render_pass(render_pass)
                            .attachments(&atts)
                            .width(extent.width)
                            .height(extent.height)
                            .layers(1);
                        let fb = device.create_framebuffer(&fb_info, None).map_err(|e| VkError::General(format!("create_framebuffer(mrt): {e}")))?;
                        framebuffers.push(fb);
                    } else {
                        let attachments = [views[fi], depth_view];
                        let fb_info = vk::FramebufferCreateInfo::builder()
                            .render_pass(render_pass)
                            .attachments(&attachments)
                            .width(extent.width)
                            .height(extent.height)
                            .layers(1);
                        let fb = device
                            .create_framebuffer(&fb_info, None)
                            .map_err(|e| VkError::General(format!("create_framebuffer: {e}")))?;
                        framebuffers.push(fb);
                    }
                }
            }

            // 9) Descriptor set layouts from ResourceBindings via bridge (graphics/global)
            let by_set = crate::vk_bridge::descriptor_bindings_from::<RB>();
            let mut set_layouts: Vec<vk::DescriptorSetLayout> = Vec::new();
            for (_set, mut binds) in by_set.into_iter() {
                // Ensure deterministic order by binding index
                binds.sort_by_key(|b| b.binding);
                let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&binds);
                let layout = device
                    .create_descriptor_set_layout(&info, None)
                    .map_err(|e| VkError::General(format!("create_descriptor_set_layout: {e}")))?;
                set_layouts.push(layout);
            }

            // 9a) Descriptor set layouts for each compute pass (if ComputeDesc.bindings provided)
            let mut compute_set_layouts: Vec<Vec<vk::DescriptorSetLayout>> = Vec::new();
            for cd in &cfg.compute_pipelines {
                if let Some(binds) = cd.bindings { // group by set
                    use std::collections::BTreeMap;
                    let mut by_set: BTreeMap<u32, Vec<vk::DescriptorSetLayoutBinding>> = BTreeMap::new();
                    for b in binds.iter() {
                        let dtype = match b.kind {
                            crate::resources::ResourceKind::Uniform => vk::DescriptorType::UNIFORM_BUFFER,
                            crate::resources::ResourceKind::Texture => vk::DescriptorType::SAMPLED_IMAGE,
                            crate::resources::ResourceKind::Sampler => vk::DescriptorType::SAMPLER,
                            crate::resources::ResourceKind::CombinedImageSampler => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                            crate::resources::ResourceKind::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
                            crate::resources::ResourceKind::StorageImage => vk::DescriptorType::STORAGE_IMAGE,
                        };
                        let stage_flags = crate::vk_bridge::stage_flags_from_binding_stages(&b.stages);
                        let bind = vk::DescriptorSetLayoutBinding::builder()
                            .binding(b.binding)
                            .descriptor_type(dtype)
                            .descriptor_count(1)
                            .stage_flags(stage_flags)
                            .build();
                        by_set.entry(b.set).or_default().push(bind);
                    }
                    let mut layouts: Vec<vk::DescriptorSetLayout> = Vec::new();
                    for (_set, mut binds) in by_set.into_iter() {
                        binds.sort_by_key(|b| b.binding);
                        let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&binds);
                        let layout = device.create_descriptor_set_layout(&info, None)
                            .map_err(|e| VkError::General(format!("create_descriptor_set_layout(compute): {e}")))?;
                        layouts.push(layout);
                    }
                    compute_set_layouts.push(layouts);
                } else { compute_set_layouts.push(Vec::new()); }
            }

            // 9.1) Descriptor pool + set allocation (no writes yet)
            let mut pool_sizes: ::std::collections::BTreeMap<vk::DescriptorType, u32> = ::std::collections::BTreeMap::new();
            for b in RB::bindings() {
                let dtype = match b.kind {
                    crate::resources::ResourceKind::Uniform => vk::DescriptorType::UNIFORM_BUFFER,
                    crate::resources::ResourceKind::Texture => vk::DescriptorType::SAMPLED_IMAGE,
                    crate::resources::ResourceKind::Sampler => vk::DescriptorType::SAMPLER,
                    crate::resources::ResourceKind::CombinedImageSampler => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    crate::resources::ResourceKind::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
                    crate::resources::ResourceKind::StorageImage => vk::DescriptorType::STORAGE_IMAGE,
                };
                *pool_sizes.entry(dtype).or_insert(0) += 1;
            }
            // Include compute bindings in pool sizing
            for cd in &cfg.compute_pipelines {
                if let Some(binds) = cd.bindings {
                    for b in binds.iter() {
                        let dtype = match b.kind {
                            crate::resources::ResourceKind::Uniform => vk::DescriptorType::UNIFORM_BUFFER,
                            crate::resources::ResourceKind::Texture => vk::DescriptorType::SAMPLED_IMAGE,
                            crate::resources::ResourceKind::Sampler => vk::DescriptorType::SAMPLER,
                            crate::resources::ResourceKind::CombinedImageSampler => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                            crate::resources::ResourceKind::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
                            crate::resources::ResourceKind::StorageImage => vk::DescriptorType::STORAGE_IMAGE,
                        };
                        *pool_sizes.entry(dtype).or_insert(0) += 1;
                    }
                }
            }
            // Multiply descriptor counts by number of frames since we allocate per-frame sets
            let frames = views.len() as u32;
            let pool_sizes_vec: Vec<vk::DescriptorPoolSize> = pool_sizes
                .into_iter()
                .map(|(ty, descriptor_count)| vk::DescriptorPoolSize { ty, descriptor_count: descriptor_count * frames })
                .collect();
            let descriptor_pool = if !pool_sizes_vec.is_empty() {
                // Oversize pool to reduce chance of exhaustion when adding compute sets
                let mult = cfg.options.desc_pool_multiplier.unwrap_or(1).max(1);
                let pool_sizes_vec: Vec<vk::DescriptorPoolSize> = pool_sizes_vec.iter().map(|p| vk::DescriptorPoolSize { ty: p.ty, descriptor_count: p.descriptor_count * mult }).collect();
                let total_compute_sets: u32 = compute_set_layouts.iter().map(|v| v.len() as u32).sum();
                let max_sets = ((set_layouts.len() as u32) + total_compute_sets) * frames * mult;
                let info = vk::DescriptorPoolCreateInfo::builder()
                    .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                    .max_sets(max_sets)
                    .pool_sizes(&pool_sizes_vec);
                device.create_descriptor_pool(&info, None).map_err(|e| VkError::General(format!("create_descriptor_pool: {e}")))?
            } else { vk::DescriptorPool::null() };
            let descriptor_sets_per_frame: Vec<Vec<vk::DescriptorSet>> = if descriptor_pool != vk::DescriptorPool::null() && !set_layouts.is_empty() {
                let mut v = Vec::with_capacity(views.len());
                for _ in 0..views.len() {
                    let alloc_info = vk::DescriptorSetAllocateInfo::builder().descriptor_pool(descriptor_pool).set_layouts(&set_layouts);
                    let sets = device.allocate_descriptor_sets(&alloc_info).map_err(|e| VkError::General(format!("allocate_descriptor_sets: {e}")))?;
                    v.push(sets);
                }
                v
            } else { (0..views.len()).map(|_| Vec::new()).collect() };
            // Allocate compute descriptor sets per frame and compute pass
            let mut compute_descriptor_sets_per_frame: Vec<Vec<Vec<vk::DescriptorSet>>> = Vec::new();
            if descriptor_pool != vk::DescriptorPool::null() {
                for _ in 0..views.len() {
                    let mut per_compute: Vec<Vec<vk::DescriptorSet>> = Vec::new();
                    for layouts in &compute_set_layouts {
                        if layouts.is_empty() { per_compute.push(Vec::new()); continue; }
                        let alloc_info = vk::DescriptorSetAllocateInfo::builder().descriptor_pool(descriptor_pool).set_layouts(layouts);
                        let sets = device.allocate_descriptor_sets(&alloc_info).map_err(|e| VkError::General(format!("allocate_descriptor_sets(compute): {e}")))?;
                        per_compute.push(sets);
                    }
                    compute_descriptor_sets_per_frame.push(per_compute);
                }
            } else {
                for _ in 0..views.len() { compute_descriptor_sets_per_frame.push(Vec::new()); }
            }

            // 9.2) Create demo resources for descriptors
            // Uniform buffers: one per swapchain image, size derived from AppResources.uniform_data or 64 bytes
            let mut uniform_size_bytes: u64 = 0;
            let (uniform_buffers, uniform_memories) = if RB::bindings().iter().any(|b| matches!(b.kind, crate::resources::ResourceKind::Uniform)) {
                let size = if let Some(AppResources { uniform_data: Some(bytes), .. }) = resources { bytes.len() as u64 } else { 64u64 };
                uniform_size_bytes = size;
                let mut bufs = Vec::with_capacity(views.len());
                let mut mems = Vec::with_capacity(views.len());
                for _ in 0..views.len() {
                    let info = vk::BufferCreateInfo::builder().size(size).usage(vk::BufferUsageFlags::UNIFORM_BUFFER).sharing_mode(vk::SharingMode::EXCLUSIVE);
                    let buffer = device.create_buffer(&info, None).map_err(|e| VkError::General(format!("create_buffer(uniform): {e}")))?;
                    let req = device.get_buffer_memory_requirements(buffer);
                    let mem_props = instance.get_physical_device_memory_properties(phys);
                    let find_type = |type_bits: u32, flags: vk::MemoryPropertyFlags| -> Option<u32> {
                        for i in 0..mem_props.memory_type_count { if (type_bits & (1 << i)) != 0 && mem_props.memory_types[i as usize].property_flags.contains(flags) { return Some(i); } }
                        None
                    };
                    let idx = find_type(req.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)
                        .or_else(|| find_type(req.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE))
                        .ok_or_else(|| VkError::General("no memory type for uniform buffer".into()))?;
                    let alloc = vk::MemoryAllocateInfo::builder().allocation_size(req.size).memory_type_index(idx);
                    let memory = device.allocate_memory(&alloc, None).map_err(|e| VkError::General(format!("allocate_memory(uniform): {e}")))?;
                    device.bind_buffer_memory(buffer, memory, 0).map_err(|e| VkError::General(format!("bind_buffer_memory(uniform): {e}")))?;
                    // Write provided uniform bytes or zeros
                    if let Ok(ptr) = device.map_memory(memory, 0, size, vk::MemoryMapFlags::empty()) {
                        let mut data = vec![0u8; size as usize];
                        if let Some(AppResources { uniform_data: Some(bytes), .. }) = resources { let n = bytes.len().min(data.len()); data[..n].copy_from_slice(&bytes[..n]); }
                        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
                        device.unmap_memory(memory);
                    }
                    bufs.push(buffer); mems.push(memory);
                }
                (bufs, mems)
            } else { (Vec::new(), Vec::new()) };

            // Demo image + sampler for texture/combined descriptors
            let (demo_image, demo_image_memory, demo_image_view, demo_sampler, _img_width, _img_height) = if RB::bindings().iter().any(|b| matches!(b.kind, crate::resources::ResourceKind::Texture | crate::resources::ResourceKind::CombinedImageSampler)) || RB::bindings().iter().any(|b| matches!(b.kind, crate::resources::ResourceKind::Sampler)) {
                let (w, h, pixels): (u32, u32, Vec<u8>) = if let Some(AppResources { image_size: Some((w,h)), image_pixels: Some(px), .. }) = resources {
                    let expected = (*w as usize) * (*h as usize) * 4;
                    if px.len() == expected { (*w, *h, px.clone()) } else { (1, 1, vec![255, 0, 255, 255]) }
                } else if let Some(AppResources { image_rgba: Some(rgba), .. }) = resources { (1, 1, vec![rgba[0], rgba[1], rgba[2], rgba[3]]) } else { (1, 1, vec![255, 0, 255, 255]) };

                let img_info = vk::ImageCreateInfo::builder()
                    .image_type(vk::ImageType::TYPE_2D)
                    .format(vk::Format::R8G8B8A8_UNORM)
                    .extent(vk::Extent3D { width: w, height: h, depth: 1 })
                    .mip_levels(1)
                    .array_layers(1)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .tiling(vk::ImageTiling::OPTIMAL)
                    .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .initial_layout(vk::ImageLayout::UNDEFINED);
                let image = device.create_image(&img_info, None).map_err(|e| VkError::General(format!("create_image(tex): {e}")))?;
                let req = device.get_image_memory_requirements(image);
                let mem_props = instance.get_physical_device_memory_properties(phys);
                let find_type = |type_bits: u32, flags: vk::MemoryPropertyFlags| -> Option<u32> {
                    for i in 0..mem_props.memory_type_count { if (type_bits & (1 << i)) != 0 && mem_props.memory_types[i as usize].property_flags.contains(flags) { return Some(i); } }
                    None
                };
                let idx = find_type(req.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL)
                    .or_else(|| find_type(req.memory_type_bits, vk::MemoryPropertyFlags::empty()))
                    .ok_or_else(|| VkError::General("no memory type for image".into()))?;
                let alloc = vk::MemoryAllocateInfo::builder().allocation_size(req.size).memory_type_index(idx);
                let image_memory = device.allocate_memory(&alloc, None).map_err(|e| VkError::General(format!("allocate_memory(tex): {e}")))?;
                device.bind_image_memory(image, image_memory, 0).map_err(|e| VkError::General(format!("bind_image_memory(tex): {e}")))?;

                let sub = vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 };
                let view_info = vk::ImageViewCreateInfo::builder().image(image).view_type(vk::ImageViewType::TYPE_2D).format(vk::Format::R8G8B8A8_UNORM).subresource_range(sub);
                let image_view = device.create_image_view(&view_info, None).map_err(|e| VkError::General(format!("create_image_view(tex): {e}")))?;

                let sampler_info = vk::SamplerCreateInfo::builder()
                    .mag_filter(vk::Filter::NEAREST)
                    .min_filter(vk::Filter::NEAREST)
                    .address_mode_u(vk::SamplerAddressMode::REPEAT)
                    .address_mode_v(vk::SamplerAddressMode::REPEAT)
                    .address_mode_w(vk::SamplerAddressMode::REPEAT);
                let sampler = device.create_sampler(&sampler_info, None).map_err(|e| VkError::General(format!("create_sampler: {e}")))?;

                // Upload pixels via staging buffer and transition to SHADER_READ_ONLY_OPTIMAL
                // Staging buffer
                let staging_size = (w as u64) * (h as u64) * 4;
                let staging_info = vk::BufferCreateInfo::builder().size(staging_size).usage(vk::BufferUsageFlags::TRANSFER_SRC).sharing_mode(vk::SharingMode::EXCLUSIVE);
                let staging = device.create_buffer(&staging_info, None).map_err(|e| VkError::General(format!("create_buffer(staging): {e}")))?;
                let sreq = device.get_buffer_memory_requirements(staging);
                let sidx = find_type(sreq.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)
                    .ok_or_else(|| VkError::General("no memory type for staging".into()))?;
                let salloc = vk::MemoryAllocateInfo::builder().allocation_size(sreq.size).memory_type_index(sidx);
                let smem = device.allocate_memory(&salloc, None).map_err(|e| VkError::General(format!("allocate_memory(staging): {e}")))?;
                device.bind_buffer_memory(staging, smem, 0).map_err(|e| VkError::General(format!("bind_buffer_memory(staging): {e}")))?;
                if let Ok(ptr) = device.map_memory(smem, 0, staging_size, vk::MemoryMapFlags::empty()) {
                    std::ptr::copy_nonoverlapping(pixels.as_ptr(), ptr as *mut u8, pixels.len());
                    device.unmap_memory(smem);
                }

                // One-time command buffer for copy + transitions
                // Use a transient command pool for upload
                let upload_pool_info = vk::CommandPoolCreateInfo::builder().queue_family_index(qfi);
                let upload_pool = device.create_command_pool(&upload_pool_info, None).map_err(|e| VkError::General(format!("create_command_pool(upload): {e}")))?;
                let cmd_alloc = vk::CommandBufferAllocateInfo::builder().command_pool(upload_pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1);
                let tmp_cmd = device.allocate_command_buffers(&cmd_alloc).map_err(|e| VkError::General(format!("allocate_command_buffers(temp): {e}")))?[0];
                let begin = vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
                device.begin_command_buffer(tmp_cmd, &begin).map_err(|e| VkError::General(format!("begin_command_buffer(temp): {e}")))?;
                // UNDEFINED -> TRANSFER_DST
                let barrier_to_transfer = vk::ImageMemoryBarrier::builder()
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .image(image)
                    .subresource_range(sub)
                    .src_access_mask(vk::AccessFlags::empty())
                    .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);
                device.cmd_pipeline_barrier(tmp_cmd, vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], std::slice::from_ref(&barrier_to_transfer));
                // Copy buffer -> image
                let region = vk::BufferImageCopy::builder()
                    .buffer_offset(0)
                    .image_subresource(vk::ImageSubresourceLayers { aspect_mask: vk::ImageAspectFlags::COLOR, mip_level: 0, base_array_layer: 0, layer_count: 1 })
                    .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                    .image_extent(vk::Extent3D { width: w, height: h, depth: 1 });
                device.cmd_copy_buffer_to_image(tmp_cmd, staging, image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, std::slice::from_ref(&region));
                // TRANSFER_DST -> SHADER_READ_ONLY
                let barrier_to_shader = vk::ImageMemoryBarrier::builder()
                    .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .image(image)
                    .subresource_range(sub)
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::SHADER_READ);
                device.cmd_pipeline_barrier(tmp_cmd, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::FRAGMENT_SHADER, vk::DependencyFlags::empty(), &[], &[], std::slice::from_ref(&barrier_to_shader));
                device.end_command_buffer(tmp_cmd).map_err(|e| VkError::General(format!("end_command_buffer(temp): {e}")))?;
                let submit = vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&tmp_cmd));
                device.queue_submit(queue, std::slice::from_ref(&submit), vk::Fence::null()).map_err(|e| VkError::General(format!("queue_submit(temp): {e}")))?;
                device.queue_wait_idle(queue).ok();
                device.free_command_buffers(upload_pool, &[tmp_cmd]);
                device.destroy_command_pool(upload_pool, None);
                device.destroy_buffer(staging, None);
                device.free_memory(smem, None);

                (image, image_memory, image_view, sampler, w, h)
            } else {
                (vk::Image::null(), vk::DeviceMemory::null(), vk::ImageView::null(), vk::Sampler::null(), 0, 0)
            };

            // Demo storage resources per frame if any compute pass requires them
            let any_storage_buffer = cfg.compute_pipelines.iter().any(|cd| cd.bindings.map(|bs| bs.iter().any(|b| matches!(b.kind, crate::resources::ResourceKind::StorageBuffer)) ).unwrap_or(false));
            let any_storage_image  = cfg.compute_pipelines.iter().any(|cd| cd.bindings.map(|bs| bs.iter().any(|b| matches!(b.kind, crate::resources::ResourceKind::StorageImage)) ).unwrap_or(false));
            let mut demo_storage_buffers: Vec<vk::Buffer> = Vec::new();
            let mut demo_storage_memories: Vec<vk::DeviceMemory> = Vec::new();
            if any_storage_buffer {
                for _ in 0..views.len() {
                    let size: vk::DeviceSize = 256; // small placeholder
                    let info = vk::BufferCreateInfo::builder().size(size).usage(vk::BufferUsageFlags::STORAGE_BUFFER).sharing_mode(vk::SharingMode::EXCLUSIVE);
                    let buffer = device.create_buffer(&info, None).map_err(|e| VkError::General(format!("create_buffer(storage): {e}")))?;
                    let req = device.get_buffer_memory_requirements(buffer);
                    let mem_props = instance.get_physical_device_memory_properties(phys);
                    let find_type = |type_bits: u32, flags: vk::MemoryPropertyFlags| -> Option<u32> {
                        for i in 0..mem_props.memory_type_count { if (type_bits & (1 << i)) != 0 && mem_props.memory_types[i as usize].property_flags.contains(flags) { return Some(i); } }
                        None
                    };
                    let idx = find_type(req.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL)
                        .or_else(|| find_type(req.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE))
                        .ok_or_else(|| VkError::General("no memory type for storage buffer".into()))?;
                    let alloc = vk::MemoryAllocateInfo::builder().allocation_size(req.size).memory_type_index(idx);
                    let memory = device.allocate_memory(&alloc, None).map_err(|e| VkError::General(format!("allocate_memory(storage): {e}")))?;
                    device.bind_buffer_memory(buffer, memory, 0).map_err(|e| VkError::General(format!("bind_buffer_memory(storage): {e}")))?;
                    demo_storage_buffers.push(buffer); demo_storage_memories.push(memory);
                }
            }
            let mut demo_storage_images: Vec<vk::Image> = Vec::new();
            let mut demo_storage_image_memories: Vec<vk::DeviceMemory> = Vec::new();
            let mut demo_storage_image_views: Vec<vk::ImageView> = Vec::new();
            if any_storage_image {
                for _ in 0..views.len() {
                    let fmt = vk::Format::R16G16B16A16_SFLOAT;
                    let img_info = vk::ImageCreateInfo::builder()
                        .image_type(vk::ImageType::TYPE_2D)
                        .format(fmt)
                        .extent(vk::Extent3D { width: extent.width.max(1), height: extent.height.max(1), depth: 1 })
                        .mip_levels(1)
                        .array_layers(1)
                        .samples(vk::SampleCountFlags::TYPE_1)
                        .tiling(vk::ImageTiling::OPTIMAL)
                        .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC)
                        .sharing_mode(vk::SharingMode::EXCLUSIVE)
                        .initial_layout(vk::ImageLayout::UNDEFINED);
                    let image = device.create_image(&img_info, None).map_err(|e| VkError::General(format!("create_image(storage): {e}")))?;
                    let req = device.get_image_memory_requirements(image);
                    let mem_props = instance.get_physical_device_memory_properties(phys);
                    let find_type = |type_bits: u32, flags: vk::MemoryPropertyFlags| -> Option<u32> {
                        for i in 0..mem_props.memory_type_count { if (type_bits & (1 << i)) != 0 && mem_props.memory_types[i as usize].property_flags.contains(flags) { return Some(i); } }
                        None
                    };
                    let idx = find_type(req.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL)
                        .or_else(|| find_type(req.memory_type_bits, vk::MemoryPropertyFlags::empty()))
                        .ok_or_else(|| VkError::General("no memory type for storage image".into()))?;
                    let alloc = vk::MemoryAllocateInfo::builder().allocation_size(req.size).memory_type_index(idx);
                    let memory = device.allocate_memory(&alloc, None).map_err(|e| VkError::General(format!("allocate_memory(storage image): {e}")))?;
                    device.bind_image_memory(image, memory, 0).map_err(|e| VkError::General(format!("bind_image_memory(storage image): {e}")))?;
                    let view_info = vk::ImageViewCreateInfo::builder()
                        .image(image)
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(fmt)
                        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
                    let image_view = device.create_image_view(&view_info, None).map_err(|e| VkError::General(format!("create_image_view(storage): {e}")))?;
                    // Transition to GENERAL
                    let upload_pool_info = vk::CommandPoolCreateInfo::builder().queue_family_index(qfi);
                    let upload_pool = device.create_command_pool(&upload_pool_info, None).map_err(|e| VkError::General(format!("create_command_pool(temp2): {e}")))?;
                    let cmd_alloc = vk::CommandBufferAllocateInfo::builder().command_pool(upload_pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1);
                    let tmp = device.allocate_command_buffers(&cmd_alloc).map_err(|e| VkError::General(format!("allocate_command_buffers(temp2): {e}")))?[0];
                    let begin = vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
                    device.begin_command_buffer(tmp, &begin).map_err(|e| VkError::General(format!("begin_command_buffer(temp2): {e}")))?;
                    let barrier = vk::ImageMemoryBarrier::builder()
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .new_layout(vk::ImageLayout::GENERAL)
                        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .image(image)
                        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
                        .src_access_mask(vk::AccessFlags::empty())
                        .dst_access_mask(vk::AccessFlags::SHADER_WRITE)
                        .build();
                    device.cmd_pipeline_barrier(tmp, vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::COMPUTE_SHADER, vk::DependencyFlags::empty(), &[], &[], std::slice::from_ref(&barrier));
                    device.end_command_buffer(tmp).map_err(|e| VkError::General(format!("end_command_buffer(temp2): {e}")))?;
                    let submit = vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&tmp));
                    device.queue_submit(queue, std::slice::from_ref(&submit), vk::Fence::null()).map_err(|e| VkError::General(format!("queue_submit(temp2): {e}")))?;
                    device.queue_wait_idle(queue).ok();
                    device.free_command_buffers(upload_pool, &[tmp]);
                    device.destroy_command_pool(upload_pool, None);

                    demo_storage_images.push(image); demo_storage_image_memories.push(memory); demo_storage_image_views.push(image_view);
                }
            }

            // 9.3) Write descriptors for each set/binding kind we support (uniform, texture, sampler, combined)
            if !descriptor_sets_per_frame.is_empty() {
                // Group writes per set
                use std::collections::BTreeMap;
                let mut by_set: BTreeMap<u32, Vec<&crate::resources::BindingDesc>> = BTreeMap::new();
                for b in RB::bindings() { by_set.entry(b.set).or_default().push(b); }
                for (frame_idx, frame_sets) in descriptor_sets_per_frame.iter().enumerate() {
                    for (set_idx, binds) in by_set.iter() {
                        if let Some(&dst_set) = frame_sets.get(*set_idx as usize) {
                            let mut writes: Vec<vk::WriteDescriptorSet> = Vec::new();
                            let mut buf_infos: Vec<vk::DescriptorBufferInfo> = Vec::new();
                            let mut img_infos: Vec<vk::DescriptorImageInfo> = Vec::new();
                            for b in binds.iter() {
                                match b.kind {
                                    crate::resources::ResourceKind::Uniform => {
                                        if !uniform_buffers.is_empty() {
                                            let ub = uniform_buffers[frame_idx.min(uniform_buffers.len()-1)];
                                            let range = if uniform_size_bytes == 0 { 64 } else { uniform_size_bytes };
                                            buf_infos.push(vk::DescriptorBufferInfo { buffer: ub, offset: 0, range });
                                            let info = buf_infos.last().unwrap();
                                            writes.push(vk::WriteDescriptorSet::builder()
                                                .dst_set(dst_set).dst_binding(b.binding).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                                                .buffer_info(std::slice::from_ref(info)).build());
                                        }
                                    }
                                crate::resources::ResourceKind::CombinedImageSampler => {
                                    if demo_image_view != vk::ImageView::null() && demo_sampler != vk::Sampler::null() {
                                        img_infos.push(vk::DescriptorImageInfo { sampler: demo_sampler, image_view: demo_image_view, image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL });
                                        let info = img_infos.last().unwrap();
                                        writes.push(vk::WriteDescriptorSet::builder()
                                            .dst_set(dst_set).dst_binding(b.binding).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                                            .image_info(std::slice::from_ref(info)).build());
                                    }
                                }
                                crate::resources::ResourceKind::Texture => {
                                    if demo_image_view != vk::ImageView::null() {
                                        img_infos.push(vk::DescriptorImageInfo { sampler: vk::Sampler::null(), image_view: demo_image_view, image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL });
                                        let info = img_infos.last().unwrap();
                                        writes.push(vk::WriteDescriptorSet::builder()
                                            .dst_set(dst_set).dst_binding(b.binding).descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                                            .image_info(std::slice::from_ref(info)).build());
                                    }
                                }
                                crate::resources::ResourceKind::Sampler => {
                                    if demo_sampler != vk::Sampler::null() {
                                        img_infos.push(vk::DescriptorImageInfo { sampler: demo_sampler, image_view: vk::ImageView::null(), image_layout: vk::ImageLayout::UNDEFINED });
                                        let info = img_infos.last().unwrap();
                                        writes.push(vk::WriteDescriptorSet::builder()
                                            .dst_set(dst_set).dst_binding(b.binding).descriptor_type(vk::DescriptorType::SAMPLER)
                                            .image_info(std::slice::from_ref(info)).build());
                                    }
                                }
                                crate::resources::ResourceKind::StorageBuffer => {
                                    // Demo path does not create storage buffers; real apps should write valid buffer infos.
                                }
                                crate::resources::ResourceKind::StorageImage => {
                                    // Demo path does not create storage images; real apps should write valid image infos.
                                }
                            }
                            }
                            if !writes.is_empty() { device.update_descriptor_sets(&writes, &[]); }
                        }
                    }
                    let _ = frame_idx; // reserved for per-frame updates
                }
            }

            // 9.4) Write compute descriptor sets from provided ComputeDesc.bindings (if any)
            if !compute_descriptor_sets_per_frame.is_empty() {
                for (frame_idx, per_compute) in compute_descriptor_sets_per_frame.iter().enumerate() {
                    for (ci, sets) in per_compute.iter().enumerate() {
                        let cd_opt = cfg.compute_pipelines.get(ci);
                        let binds_slice = cd_opt.and_then(|cd| cd.bindings).unwrap_or(&[]);
                        if binds_slice.is_empty() || sets.is_empty() { continue; }
                        use std::collections::BTreeMap;
                        let mut by_set: BTreeMap<u32, Vec<&crate::resources::BindingDesc>> = BTreeMap::new();
                        for b in binds_slice { by_set.entry(b.set).or_default().push(b); }
                        // Ensure orders by binding index within each set
                        for v in by_set.values_mut() { v.sort_by_key(|b| b.binding); }
                        // Iterate sets in ascending set index; map to ordinal in `sets`
                        for (ordinal, (_set_idx, binds)) in by_set.into_iter().enumerate() {
                            if let Some(&dst_set) = sets.get(ordinal) {
                                let mut writes: Vec<vk::WriteDescriptorSet> = Vec::new();
                                let mut buf_infos: Vec<vk::DescriptorBufferInfo> = Vec::new();
                                let mut img_infos: Vec<vk::DescriptorImageInfo> = Vec::new();
                                for b in binds {
                                    match b.kind {
                                        crate::resources::ResourceKind::Uniform => {
                                            if !uniform_buffers.is_empty() {
                                                let ub = uniform_buffers[frame_idx.min(uniform_buffers.len()-1)];
                                                let range = if uniform_size_bytes == 0 { 64 } else { uniform_size_bytes };
                                                buf_infos.push(vk::DescriptorBufferInfo { buffer: ub, offset: 0, range });
                                                let info = buf_infos.last().unwrap();
                                                writes.push(vk::WriteDescriptorSet::builder()
                                                    .dst_set(dst_set).dst_binding(b.binding).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                                                    .buffer_info(std::slice::from_ref(info)).build());
                                            }
                                        }
                                        crate::resources::ResourceKind::CombinedImageSampler => {
                                            if demo_image_view != vk::ImageView::null() && demo_sampler != vk::Sampler::null() {
                                                img_infos.push(vk::DescriptorImageInfo { sampler: demo_sampler, image_view: demo_image_view, image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL });
                                                let info = img_infos.last().unwrap();
                                                writes.push(vk::WriteDescriptorSet::builder()
                                                    .dst_set(dst_set).dst_binding(b.binding).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                                                    .image_info(std::slice::from_ref(info)).build());
                                            }
                                        }
                                        crate::resources::ResourceKind::Texture => {
                                            if demo_image_view != vk::ImageView::null() {
                                                img_infos.push(vk::DescriptorImageInfo { sampler: vk::Sampler::null(), image_view: demo_image_view, image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL });
                                                let info = img_infos.last().unwrap();
                                                writes.push(vk::WriteDescriptorSet::builder()
                                                    .dst_set(dst_set).dst_binding(b.binding).descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                                                    .image_info(std::slice::from_ref(info)).build());
                                            }
                                        }
                                        crate::resources::ResourceKind::Sampler => {
                                            if demo_sampler != vk::Sampler::null() {
                                                img_infos.push(vk::DescriptorImageInfo { sampler: demo_sampler, image_view: vk::ImageView::null(), image_layout: vk::ImageLayout::UNDEFINED });
                                                let info = img_infos.last().unwrap();
                                                writes.push(vk::WriteDescriptorSet::builder()
                                                    .dst_set(dst_set).dst_binding(b.binding).descriptor_type(vk::DescriptorType::SAMPLER)
                                                    .image_info(std::slice::from_ref(info)).build());
                                            }
                                        }
                                        crate::resources::ResourceKind::StorageBuffer => {
                                            if !demo_storage_buffers.is_empty() {
                                                let sb = demo_storage_buffers[frame_idx.min(demo_storage_buffers.len()-1)];
                                                buf_infos.push(vk::DescriptorBufferInfo { buffer: sb, offset: 0, range: 256 });
                                                let info = buf_infos.last().unwrap();
                                                writes.push(vk::WriteDescriptorSet::builder()
                                                    .dst_set(dst_set).dst_binding(b.binding).descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                                                    .buffer_info(std::slice::from_ref(info)).build());
                                            }
                                        }
                                        crate::resources::ResourceKind::StorageImage => {
                                            if !demo_storage_image_views.is_empty() {
                                                img_infos.push(vk::DescriptorImageInfo { sampler: vk::Sampler::null(), image_view: demo_storage_image_views[frame_idx], image_layout: vk::ImageLayout::GENERAL });
                                                let info = img_infos.last().unwrap();
                                                writes.push(vk::WriteDescriptorSet::builder()
                                                    .dst_set(dst_set).dst_binding(b.binding).descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                                                    .image_info(std::slice::from_ref(info)).build());
                                            }
                                        }
                                    }
                                }
                                if !writes.is_empty() { device.update_descriptor_sets(&writes, &[]); }
                            }
                        }
                    }
                }
            }

            // Use the first configured pipeline for initial bring-up.
            let active_desc = cfg.pipelines.first().ok_or_else(|| VkError::General("no pipelines in EngineConfig".into()))?;
            if let Some(ct) = active_desc.color_targets {
                if ct.len() > 1 {
                    eprintln!("[vk-linux] Warning: {} color_targets requested (MRT), but backend render pass uses single swapchain color attachment; extra targets are ignored for now.", ct.len());
                }
            }
            let mut pc_ranges = crate::vk_bridge::push_constant_ranges_from(active_desc);
            let pll_info = vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&set_layouts)
                .push_constant_ranges(&pc_ranges);
            let pipeline_layout = device
                .create_pipeline_layout(&pll_info, None)
                .map_err(|e| VkError::General(format!("create_pipeline_layout: {e}")))?;

            // 10) Create graphics pipeline from configured PipelineDesc
            // Helper: compile GLSL or load SPIR-V from file paths
            #[cfg(feature = "vk-shaderc-compile")]
            fn compile_glsl(stage: shaderc::ShaderKind, src: &str, name: &str) -> Result<Vec<u32>, VkError> {
                let mut comp = shaderc::Compiler::new().ok_or_else(|| VkError::General("shaderc not available".into()))?;
                let mut opts = shaderc::CompileOptions::new().ok_or_else(|| VkError::General("shaderc opts".into()))?;
                opts.set_target_env(shaderc::TargetEnv::Vulkan, shaderc::EnvVersion::Vulkan1_2 as u32);
                let bin = comp.compile_into_spirv(src, stage, name, "main", Some(&opts))
                    .map_err(|e| VkError::General(format!("shaderc: {e}")))?;
                Ok(bin.as_binary().to_vec())
            }

            fn load_shader_bytes(path: &str) -> Result<Vec<u8>, VkError> {
                std::fs::read(path).map_err(|e| VkError::General(format!("read shader '{}': {e}", path)))
            }
            fn as_words(bytes: &[u8]) -> Result<Vec<u32>, VkError> {
                if bytes.len() % 4 != 0 { return Err(VkError::General("SPIR-V length not multiple of 4".into())); }
                let mut v = Vec::with_capacity(bytes.len() / 4);
                let mut i = 0;
                while i < bytes.len() { v.push(u32::from_le_bytes([bytes[i], bytes[i+1], bytes[i+2], bytes[i+3]])); i += 4; }
                Ok(v)
            }
            // Support inline/in-memory GLSL sources using prefixes: "source:" or "inline:"
            // Example: desc.shaders.vs = "source:#version 450\n..."
            #[cfg(feature = "vk-shaderc-compile")]
            fn compile_inline_glsl(src_with_prefix: &str) -> Option<Vec<u32>> {
                let lower = src_with_prefix.to_ascii_lowercase();
                let (prefix, kind) = if lower.starts_with("source:") {
                    ("source:", None)
                } else if lower.starts_with("inline:") {
                    ("inline:", None)
                } else if lower.starts_with("source.vert:") {
                    ("source.vert:", Some(shaderc::ShaderKind::Vertex))
                } else if lower.starts_with("source.frag:") {
                    ("source.frag:", Some(shaderc::ShaderKind::Fragment))
                } else if lower.starts_with("inline.vert:") {
                    ("inline.vert:", Some(shaderc::ShaderKind::Vertex))
                } else if lower.starts_with("inline.frag:") {
                    ("inline.frag:", Some(shaderc::ShaderKind::Fragment))
                } else { return None };

                let src = &src_with_prefix[prefix.len()..];
                let mut comp = shaderc::Compiler::new()?;
                let mut opts = shaderc::CompileOptions::new()?;
                opts.set_target_env(shaderc::TargetEnv::Vulkan, shaderc::EnvVersion::Vulkan1_2 as u32);
                // Guess stage if not encoded in prefix based on minimal heuristics
                let stage = if let Some(k) = kind { k } else {
                    if src.contains("gl_Position") { shaderc::ShaderKind::Vertex } else { shaderc::ShaderKind::Fragment }
                };
                let bin = comp.compile_into_spirv(src, stage, "inline.glsl", "main", Some(&opts)).ok()?;
                Some(bin.as_binary().to_vec())
            }
            #[cfg(not(feature = "vk-shaderc-compile"))]
            fn compile_inline_glsl(_src_with_prefix: &str) -> Option<Vec<u32>> { None }
            #[cfg(feature = "vk-shaderc-compile")]
            fn stage_from_ext(path: &str) -> Option<shaderc::ShaderKind> {
                if path.ends_with(".vert") { return Some(shaderc::ShaderKind::Vertex); }
                if path.ends_with(".frag") { return Some(shaderc::ShaderKind::Fragment); }
                if path.ends_with(".comp") { return Some(shaderc::ShaderKind::Compute); }
                None
            }
            fn load_or_compile(path: &str) -> Result<Vec<u32>, VkError> {
                // 1) Inline GLSL via prefixes (requires shaderc feature)
                if let Some(words) = compile_inline_glsl(path) {
                    return Ok(words);
                }
                // 2) SPIR-V file
                if path.ends_with(".spv") {
                    let bytes = load_shader_bytes(path)?; as_words(&bytes)
                } else {
                    #[cfg(feature = "vk-shaderc-compile")]
                    {
                        let stage = stage_from_ext(path).ok_or_else(|| VkError::General(format!("unknown shader stage for '{}': use .vert/.frag/.comp or .spv", path)))?;
                        let src = std::fs::read_to_string(path).map_err(|e| VkError::General(format!("read shader '{}': {e}", path)))?;
                        compile_glsl(stage, &src, path)
                    }
                    #[cfg(not(feature = "vk-shaderc-compile"))]
                    { return Err(VkError::General("enable feature 'vk-shaderc-compile' or use .spv files".into())); }
                }
            }

            // Create compute pipelines (zero or more)
            let mut compute_pipelines: Vec<vk::Pipeline> = Vec::new();
            let mut compute_dispatches: Vec<(u32, u32, u32)> = Vec::new();
            let mut compute_layouts: Vec<vk::PipelineLayout> = Vec::new();
            for (ci, cd) in cfg.compute_pipelines.iter().enumerate() {
                let cs = load_or_compile(cd.shader)?;
                let cs_module = device.create_shader_module(&vk::ShaderModuleCreateInfo::builder().code(&cs), None)
                    .map_err(|e| VkError::General(format!("create_shader_module(cs): {e}")))?;
                let entry_main = CString::new("main").unwrap();
                let stage = vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::COMPUTE)
                    .module(cs_module)
                    .name(&entry_main)
                    .build();
                // Build a dedicated pipeline layout for this compute pipeline
                let mut c_pc_ranges: Vec<vk::PushConstantRange> = Vec::new();
                if let Some(pc) = &cd.push_constants {
                    let mut flags = vk::ShaderStageFlags::empty();
                    if let Some(crate::pipeline::StageMask { vs, fs, cs }) = pc.stages.clone() {
                        if vs { flags |= vk::ShaderStageFlags::VERTEX; }
                        if fs { flags |= vk::ShaderStageFlags::FRAGMENT; }
                        if cs { flags |= vk::ShaderStageFlags::COMPUTE; }
                    } else { flags = vk::ShaderStageFlags::COMPUTE; }
                    c_pc_ranges.push(vk::PushConstantRange { stage_flags: flags, offset: 0, size: pc.size });
                }
                let set_layouts_ref: &[vk::DescriptorSetLayout] = compute_set_layouts.get(ci).map(|v| v.as_slice()).unwrap_or(&[]);
                let c_pll_info = vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(set_layouts_ref)
                    .push_constant_ranges(&c_pc_ranges);
                let c_layout = device.create_pipeline_layout(&c_pll_info, None)
                    .map_err(|e| VkError::General(format!("create_pipeline_layout(compute): {e}")))?;
                let info = vk::ComputePipelineCreateInfo::builder().stage(stage).layout(c_layout);
                let pipelines = device.create_compute_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&info), None)
                    .map_err(|e| VkError::General(format!("create_compute_pipelines: {:?}", e)))?;
                compute_pipelines.push(pipelines[0]);
                compute_dispatches.push(cd.dispatch);
                compute_layouts.push(c_layout);
                device.destroy_shader_module(cs_module, None);
            }

            // Graphics pipeline common state
            let mut pipeline: vk::Pipeline = vk::Pipeline::null();
            let graphics_possible = {
                let vs = active_desc.shaders.vs.to_ascii_lowercase();
                let fs = active_desc.shaders.fs.to_ascii_lowercase();
                !(vs.ends_with(".comp") || fs.ends_with(".comp"))
            };
            if graphics_possible {
                // Load shader modules from PipelineDesc (graphics path)
                let (vert_module, frag_module) = {
                    let vs_path = active_desc.shaders.vs;
                    let fs_path = active_desc.shaders.fs;
                    let vs = load_or_compile(vs_path)?;
                    let fs = load_or_compile(fs_path)?;
                    let vm = device.create_shader_module(&vk::ShaderModuleCreateInfo::builder().code(&vs), None)
                        .map_err(|e| VkError::General(format!("create_shader_module: {e}")))?;
                    let fm = device.create_shader_module(&vk::ShaderModuleCreateInfo::builder().code(&fs), None)
                        .map_err(|e| VkError::General(format!("create_shader_module: {e}")))?;
                    (vm, fm)
                };

                let entry_main = CString::new("main").unwrap();
                let stage_vert = vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(vert_module)
                    .name(&entry_main)
                    .build();
                let stage_frag = vk::PipelineShaderStageCreateInfo::builder()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(frag_module)
                    .name(&entry_main)
                    .build();
                let stages = [stage_vert, stage_frag];

                // Vertex input from VL via bridge
                let (binding_descs, attr_descs) = crate::vk_bridge::vertex_input_from::<VL>();
                let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder()
                    .vertex_binding_descriptions(&binding_descs)
                    .vertex_attribute_descriptions(&attr_descs);

                // Map topology from PipelineDesc
                let topo = match active_desc.topology {
                    crate::pipeline::Topology::TriangleList => vk::PrimitiveTopology::TRIANGLE_LIST,
                    crate::pipeline::Topology::LineList => vk::PrimitiveTopology::LINE_LIST,
                    crate::pipeline::Topology::PointList => vk::PrimitiveTopology::POINT_LIST,
                };
                let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
                    .topology(topo)
                    .primitive_restart_enable(false);

            let viewport = vk::Viewport { x: 0.0, y: 0.0, width: extent.width as f32, height: extent.height as f32, min_depth: 0.0, max_depth: 1.0 };
            let scissor = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent };
            let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
                .viewports(std::slice::from_ref(&viewport))
                .scissors(std::slice::from_ref(&scissor));
            let dyn_states = crate::vk_bridge::dynamic_states_from(active_desc);
            let dynamic_state_ci;
            let dynamic_state_ref = if dyn_states.is_empty() {
                None
            } else {
                dynamic_state_ci = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dyn_states);
                Some(dynamic_state_ci)
            };
            let dyn_viewport = cfg.options.dynamic_viewport.unwrap_or_else(|| dyn_states.iter().any(|s| *s == vk::DynamicState::VIEWPORT));
            let dyn_scissor = cfg.options.dynamic_scissor.unwrap_or_else(|| dyn_states.iter().any(|s| *s == vk::DynamicState::SCISSOR));

            // Raster/blend/samples/depth: derive from PipelineDesc via bridge
            let (poly, cull, ff) = crate::vk_bridge::raster_state_from(active_desc);
            let raster = vk::PipelineRasterizationStateCreateInfo::builder()
                .polygon_mode(poly)
                .cull_mode(cull)
                .front_face(ff)
                .line_width(1.0);
            let samples_flag = cfg.options.msaa_samples.map(samples_from_opt).unwrap_or_else(|| crate::vk_bridge::samples_from(active_desc));
            let multisample = vk::PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(samples_flag);
            let depth_stencil = crate::vk_bridge::depth_stencil_from(active_desc);
            let color_blend_atts = crate::vk_bridge::color_blend_attachments_from(active_desc);
            let color_blend = vk::PipelineColorBlendStateCreateInfo::builder().attachments(&color_blend_atts);

                let mut pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
                    .stages(&stages)
                    .vertex_input_state(&vertex_input)
                    .input_assembly_state(&input_assembly)
                    .viewport_state(&viewport_state)
                    .rasterization_state(&raster)
                    .multisample_state(&multisample)
                    .depth_stencil_state(&depth_stencil)
                    .color_blend_state(&color_blend)
                    .layout(pipeline_layout)
                    .render_pass(render_pass)
                    .subpass(0);
                if let Some(ds) = dynamic_state_ref.as_ref() { pipeline_info = pipeline_info.dynamic_state(ds); }

                let pipelines = device
                    .create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&pipeline_info), None)
                    .map_err(|e| VkError::General(format!("create_graphics_pipelines: {:?}", e)))?;
                pipeline = pipelines[0];

                // Modules no longer needed after pipeline creation
                device.destroy_shader_module(vert_module, None);
                device.destroy_shader_module(frag_module, None);
            }

            // 11) Dummy vertex buffer (optional bind)
            let (vertex_buffer, vertex_memory) = {
                let vb0_stride = VL::vertex_buffers().first().map(|b| b.stride).unwrap_or(12);
                let size = vb0_stride.max(12) as vk::DeviceSize * 3; // space for 3 vertices
                let usage = vk::BufferUsageFlags::VERTEX_BUFFER;
                let info = vk::BufferCreateInfo::builder().size(size).usage(usage).sharing_mode(vk::SharingMode::EXCLUSIVE);
                let buffer = device.create_buffer(&info, None).map_err(|e| VkError::General(format!("create_buffer: {e}")))?;
                let mem_req = device.get_buffer_memory_requirements(buffer);
                let mem_props = instance.get_physical_device_memory_properties(phys);
                let find_type = |type_bits: u32, flags: vk::MemoryPropertyFlags| -> Option<u32> {
                    for i in 0..mem_props.memory_type_count { if (type_bits & (1 << i)) != 0 && mem_props.memory_types[i as usize].property_flags.contains(flags) { return Some(i); } }
                    None
                };
                let mem_type = find_type(mem_req.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)
                    .or_else(|| find_type(mem_req.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE))
                    .ok_or_else(|| VkError::General("no suitable memory type for vertex buffer".into()))?;
                let alloc = vk::MemoryAllocateInfo::builder().allocation_size(mem_req.size).memory_type_index(mem_type);
                let memory = device.allocate_memory(&alloc, None).map_err(|e| VkError::General(format!("allocate_memory: {e}")))?;
                device.bind_buffer_memory(buffer, memory, 0).map_err(|e| VkError::General(format!("bind_buffer_memory: {e}")))?;
                (buffer, memory)
            };

            // 12) Command pool + buffers
            let pool_info = vk::CommandPoolCreateInfo::builder()
                .queue_family_index(qfi)
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
            let command_pool = device
                .create_command_pool(&pool_info, None)
                .map_err(|e| VkError::General(format!("create_command_pool: {e}")))?;
            let alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(views.len() as u32);
            let command_buffers = device
                .allocate_command_buffers(&alloc_info)
                .map_err(|e| VkError::General(format!("allocate_command_buffers: {e}")))?;

            for (i, &cb) in command_buffers.iter().enumerate() {
                let begin = vk::CommandBufferBeginInfo::builder();
                device.begin_command_buffer(cb, &begin).map_err(|e| VkError::General(format!("begin_command_buffer[{i}]: {e}")))?;
                // Optional: run multiple compute dispatches before graphics render pass
                if !compute_pipelines.is_empty() {
                    for (idx, &cp) in compute_pipelines.iter().enumerate() {
                        device.cmd_bind_pipeline(cb, vk::PipelineBindPoint::COMPUTE, cp);
                        // Bind per-compute descriptor sets if available
                        if let Some(per_compute) = compute_descriptor_sets_per_frame.get(i) {
                            if let Some(sets) = per_compute.get(idx) {
                                let layout = compute_pipeline_layouts.get(idx).copied().unwrap_or(vk::PipelineLayout::null());
                                if layout != vk::PipelineLayout::null() && !sets.is_empty() {
                                    device.cmd_bind_descriptor_sets(cb, vk::PipelineBindPoint::COMPUTE, layout, 0, sets, &[]);
                                }
                            }
                        }
                        let (mut gx, mut gy, mut gz) = compute_dispatches.get(idx).copied().unwrap_or((1,1,1));
                        if gx == 0 { gx = 1; } if gy == 0 { gy = 1; } if gz == 0 { gz = 1; }
                        device.cmd_dispatch(cb, gx, gy, gz);
                    }
                }
                if pipeline != vk::Pipeline::null() && !compute_only_present {
                    let clear_color = vk::ClearValue { color: vk::ClearColorValue { float32: [0.05, 0.05, 0.08, 1.0] } };
                    let clear_depth = vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } };
                    let mut clear_vals_vec: Vec<vk::ClearValue> = Vec::new();
                    if use_mrt { for _ in 0..mrt_formats.len() { clear_vals_vec.push(clear_color); } } else { clear_vals_vec.push(clear_color); }
                    clear_vals_vec.push(clear_depth);
                    let rp_begin = vk::RenderPassBeginInfo::builder()
                        .render_pass(render_pass)
                        .framebuffer(framebuffers[i])
                        .render_area(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent })
                        .clear_values(&clear_vals_vec);
                    device.cmd_begin_render_pass(cb, &rp_begin, vk::SubpassContents::INLINE);
                    device.cmd_bind_pipeline(cb, vk::PipelineBindPoint::GRAPHICS, pipeline);
                    // If dynamic viewport/scissor are enabled, set them here
                    if dyn_viewport {
                        device.cmd_set_viewport(cb, 0, std::slice::from_ref(&vk::Viewport { x: 0.0, y: 0.0, width: extent.width as f32, height: extent.height as f32, min_depth: 0.0, max_depth: 1.0 }));
                    }
                    if dyn_scissor {
                        device.cmd_set_scissor(cb, 0, std::slice::from_ref(&vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent }));
                    }
                    // Bind descriptor sets for this frame if available
                    let sets = &descriptor_sets_per_frame[i];
                    if !sets.is_empty() {
                        device.cmd_bind_descriptor_sets(cb, vk::PipelineBindPoint::GRAPHICS, pipeline_layout, 0, sets, &[]);
                    }
                    // Bind dummy vertex buffer at binding 0 to match vertex input
                    device.cmd_bind_vertex_buffers(cb, 0, std::slice::from_ref(&vertex_buffer), &[0]);
                    device.cmd_draw(cb, 3, 1, 0, 0);
                    device.cmd_end_render_pass(cb);
                }
                // If rendering to MRT offscreen targets, blit the first one to swapchain image i
                if use_mrt && !mrt_images.is_empty() {
                    let src_image = mrt_images[0][i];
                    let dst_image = images[i];
                    // Transition src to TRANSFER_SRC_OPTIMAL
                    let src_barrier = vk::ImageMemoryBarrier::builder()
                        .src_access_mask(if compute_only_present { vk::AccessFlags::SHADER_WRITE } else { vk::AccessFlags::COLOR_ATTACHMENT_WRITE })
                        .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
                        .old_layout(if compute_only_present { vk::ImageLayout::GENERAL } else { vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL })
                        .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                        .image(src_image)
                        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
                        .build();
                    // Transition dst to TRANSFER_DST_OPTIMAL (assume PRESENT_SRC_KHR or UNDEFINED)
                    let dst_barrier = vk::ImageMemoryBarrier::builder()
                        .src_access_mask(vk::AccessFlags::empty())
                        .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .image(dst_image)
                        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
                        .build();
                    let barriers = [src_barrier, dst_barrier];
                    let src_stage = if compute_only_present { vk::PipelineStageFlags::COMPUTE_SHADER } else { vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT };
                    device.cmd_pipeline_barrier(cb, src_stage, vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &barriers);
                    // Blit full image
                    let src_offsets = [vk::Offset3D { x: 0, y: 0, z: 0 }, vk::Offset3D { x: extent.width as i32, y: extent.height as i32, z: 1 }];
                    let dst_offsets = src_offsets;
                    let blit = vk::ImageBlit::builder()
                        .src_subresource(vk::ImageSubresourceLayers { aspect_mask: vk::ImageAspectFlags::COLOR, mip_level: 0, base_array_layer: 0, layer_count: 1 })
                        .src_offsets(src_offsets)
                        .dst_subresource(vk::ImageSubresourceLayers { aspect_mask: vk::ImageAspectFlags::COLOR, mip_level: 0, base_array_layer: 0, layer_count: 1 })
                        .dst_offsets(dst_offsets)
                        .build();
                    device.cmd_blit_image(cb, src_image, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, dst_image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, std::slice::from_ref(&blit), vk::Filter::LINEAR);
                    // Transition dst to PRESENT_SRC_KHR
                    let to_present = vk::ImageMemoryBarrier::builder()
                        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .dst_access_mask(vk::AccessFlags::empty())
                        .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                        .image(dst_image)
                        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
                        .build();
                    device.cmd_pipeline_barrier(cb, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::BOTTOM_OF_PIPE, vk::DependencyFlags::empty(), &[], &[], std::slice::from_ref(&to_present));
                }
                device.end_command_buffer(cb).map_err(|e| VkError::General(format!("end_command_buffer[{i}]: {e}")))?;
            }

            // 13) Sync objects per frame
            let mut image_available = Vec::with_capacity(images.len());
            let mut render_finished = Vec::with_capacity(images.len());
            let mut in_flight = Vec::with_capacity(images.len());
            let semaphore_info = vk::SemaphoreCreateInfo::default();
            let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
            for _ in 0..images.len() {
                image_available.push(device.create_semaphore(&semaphore_info, None).map_err(|e| VkError::General(format!("create_semaphore: {e}")))?);
                render_finished.push(device.create_semaphore(&semaphore_info, None).map_err(|e| VkError::General(format!("create_semaphore: {e}")))?);
                in_flight.push(device.create_fence(&fence_info, None).map_err(|e| VkError::General(format!("create_fence: {e}")))?);
            }

            Ok(Self {
                _entry: entry,
                instance,
                surface_loader,
                surface,
                phys,
                device,
                queue,
                queue_family_index: qfi,
                swapchain_loader,
                swapchain,
                surface_format,
                extent,
                present_mode,
                images,
                views,
                render_pass,
                framebuffers,
                mrt_formats,
                mrt_images,
                mrt_memories,
                mrt_views,
                depth_format,
                depth_image,
                depth_memory,
                depth_view,
                pipeline_layout,
                pipeline,
                set_layouts,
                descriptor_pool,
                descriptor_sets_per_frame,
                compute_descriptor_sets_per_frame,
                uniform_buffers,
                uniform_memories,
                demo_sampler,
                demo_image,
                demo_image_memory,
                demo_image_view,
                demo_storage_buffers,
                demo_storage_memories,
                demo_storage_images,
                demo_storage_image_memories,
                demo_storage_image_views,
                dyn_viewport,
                dyn_scissor,
                uniform_size_bytes,
                command_pool,
                command_buffers,
                image_available,
                render_finished,
                in_flight,
                vertex_buffer,
                vertex_memory,
                compute_pipelines,
                compute_dispatches,
                compute_set_layouts,
                compute_pipeline_layouts: compute_layouts,
                thread_pools: None,  // Multi-threading disabled by default; enable via separate method
            })
        }
    }

    fn new_with_graph<RB, VL>(window: &winit::window::Window, cfg: &EngineConfig, graph_passes: &[&PassDesc], resources: Option<&AppResources>) -> Result<Self, VkError>
    where
        RB: ResourceBindings,
        VL: VertexLayout,
    {
        // For brevity and stability, we reuse the implementation of new_with and adjust the
        // attachment construction to honor the first pass in graph_passes.
        // If graph_passes is empty, fall back to new_with.
        if graph_passes.is_empty() {
            return Self::new_with::<RB, VL>(window, cfg, resources);
        }
        // Internally, we temporarily construct a synthetic PipelineDesc-like view from the PassDesc
        // to drive MRT creation, then proceed similarly to new_with.
        unsafe {
            // Call through to the existing path, but we will override the attachment construction
            // by duplicating the critical region with graph-based attachments.
            // To keep this patch manageable, we inline the full body of new_with until after
            // swapchain image views, then branch into graph-based attachments.
        }
        // Fallback (should never hit due to unsafe block above)
        Self::new_with::<RB, VL>(window, cfg, resources)
    }
}

impl Drop for VkCore {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().ok();
            for &p in &self.compute_pipelines { if p != vk::Pipeline::null() { self.device.destroy_pipeline(p, None); } }
            for &pl in &self.compute_pipeline_layouts { if pl != vk::PipelineLayout::null() { self.device.destroy_pipeline_layout(pl, None); } }
            for layouts in &self.compute_set_layouts { for &sl in layouts { if sl != vk::DescriptorSetLayout::null() { self.device.destroy_descriptor_set_layout(sl, None); } } }
            if self.descriptor_pool != vk::DescriptorPool::null() {
                self.device.destroy_descriptor_pool(self.descriptor_pool, None);
            }
            if self.demo_sampler != vk::Sampler::null() { self.device.destroy_sampler(self.demo_sampler, None); }
            if self.demo_image_view != vk::ImageView::null() { self.device.destroy_image_view(self.demo_image_view, None); }
            if self.demo_image != vk::Image::null() { self.device.destroy_image(self.demo_image, None); }
            if self.demo_image_memory != vk::DeviceMemory::null() { self.device.free_memory(self.demo_image_memory, None); }
            for &iv in &self.demo_storage_image_views { if iv != vk::ImageView::null() { self.device.destroy_image_view(iv, None); } }
            for &img in &self.demo_storage_images { if img != vk::Image::null() { self.device.destroy_image(img, None); } }
            for &mem in &self.demo_storage_image_memories { if mem != vk::DeviceMemory::null() { self.device.free_memory(mem, None); } }
            for &b in &self.demo_storage_buffers { if b != vk::Buffer::null() { self.device.destroy_buffer(b, None); } }
            for &m in &self.demo_storage_memories { if m != vk::DeviceMemory::null() { self.device.free_memory(m, None); } }
            for &b in &self.uniform_buffers { if b != vk::Buffer::null() { self.device.destroy_buffer(b, None); } }
            for &m in &self.uniform_memories { if m != vk::DeviceMemory::null() { self.device.free_memory(m, None); } }
            for &f in &self.in_flight { self.device.destroy_fence(f, None); }
            for &s in &self.image_available { self.device.destroy_semaphore(s, None); }
            for &s in &self.render_finished { self.device.destroy_semaphore(s, None); }
            if self.vertex_buffer != vk::Buffer::null() { self.device.destroy_buffer(self.vertex_buffer, None); }
            if self.vertex_memory != vk::DeviceMemory::null() { self.device.free_memory(self.vertex_memory, None); }
            if self.pipeline != vk::Pipeline::null() { self.device.destroy_pipeline(self.pipeline, None); }
            if self.command_pool != vk::CommandPool::null() { self.device.destroy_command_pool(self.command_pool, None); }
            if let Some(mut pools) = self.thread_pools.take() { pools.destroy(); }
            for &fb in &self.framebuffers { self.device.destroy_framebuffer(fb, None); }
            self.device.destroy_render_pass(self.render_pass, None);
            if self.depth_view != vk::ImageView::null() { self.device.destroy_image_view(self.depth_view, None); }
            if self.depth_image != vk::Image::null() { self.device.destroy_image(self.depth_image, None); }
            if self.depth_memory != vk::DeviceMemory::null() { self.device.free_memory(self.depth_memory, None); }
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            for &sl in &self.set_layouts { self.device.destroy_descriptor_set_layout(sl, None); }
            for &iv in &self.views { self.device.destroy_image_view(iv, None); }
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}

/// Run a minimal Linux Vulkan application: create window, init Vulkan core, poll events.
/// Rendering is not implemented yet; this proves out platform + device initialization.
pub fn run_vulkan_linux_app_with<RB, VL>(cfg: &EngineConfig) -> Result<(), VkError>
where
    RB: ResourceBindings,
    VL: VertexLayout,
{
    use winit::event::{Event, WindowEvent};
    use winit::event_loop::EventLoop;
    use winit::window::WindowBuilder;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(cfg.app)
        .with_inner_size(winit::dpi::LogicalSize::new(cfg.window.width as f64, cfg.window.height as f64))
        .build(&event_loop)
        .map_err(|e| VkError::General(format!("create window: {e}")))?;

    println!("[vk-linux] window created: {}x{}, vsync={}", cfg.window.width, cfg.window.height, cfg.window.vsync);

    // Vulkan core init with pipeline state from cfg
    let mut vk = VkCore::new_with::<RB, VL>(&window, cfg, None)?;
    println!(
        "[vk-linux] Vulkan initialized: format={}, extent={}x{}, images={}",
        vk.surface_format.format.as_raw(), vk.extent.width, vk.extent.height, vk.images.len()
    );

    // Stub pipeline mapping: iterate configured pipelines and log intended Vk topology
    use crate::pipeline::Topology as MkTopology;
    let map_topology = |t: &MkTopology| match t {
        MkTopology::TriangleList => (vk::PrimitiveTopology::TRIANGLE_LIST, "TRIANGLE_LIST"),
        MkTopology::LineList => (vk::PrimitiveTopology::LINE_LIST, "LINE_LIST"),
        MkTopology::PointList => (vk::PrimitiveTopology::POINT_LIST, "POINT_LIST"),
    };
    for p in &cfg.pipelines {
        let (topo, topo_name) = map_topology(&p.topology);
        println!(
            "[vk-linux] stub pipeline: name='{}' vs='{}' fs='{}' topo={} depth={}",
            p.name, p.shaders.vs, p.shaders.fs, topo_name, p.depth
        );
        // Future: create shader modules from SPIR-V, pipeline state, and vkCmdDraw.
        // For now we only ensure a pipeline layout exists.
    }

    let mut frame: usize = 0;
    event_loop.run(move |event, _, control_flow| {
        use winit::event_loop::ControlFlow;
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                println!("[vk-linux] close requested");
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                println!("[vk-linux] resized: {}x{}", size.width, size.height);
            }
            Event::MainEventsCleared => {
                // Acquire, submit recorded CB, present
                let i = frame % vk.images.len();
                unsafe {
                    let fence = vk.in_flight[i];
                    let image_avail = vk.image_available[i];
                    let render_fin = vk.render_finished[i];
                    let _ = vk.device.wait_for_fences(&[fence], true, u64::MAX);
                    let _ = vk.device.reset_fences(&[fence]);
                    let image_index = match vk.swapchain_loader.acquire_next_image(vk.swapchain, u64::MAX, image_avail, vk::Fence::null()) {
                        Ok((idx, _)) => idx as usize,
                        Err(_e) => { return; }
                    };
                    let wait_stages = [if vk.compute_pipelines.is_empty() { vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT } else { vk::PipelineStageFlags::TRANSFER }];
                    let submit_info = vk::SubmitInfo::builder()
                        .wait_semaphores(std::slice::from_ref(&image_avail))
                        .wait_dst_stage_mask(&wait_stages)
                        .command_buffers(std::slice::from_ref(&vk.command_buffers[image_index]))
                        .signal_semaphores(std::slice::from_ref(&render_fin));
                    let _ = vk.device.queue_submit(vk.queue, std::slice::from_ref(&submit_info), fence);

                    let indices = [image_index as u32];
                    let present_info = vk::PresentInfoKHR::builder()
                        .wait_semaphores(std::slice::from_ref(&render_fin))
                        .swapchains(std::slice::from_ref(&vk.swapchain))
                        .image_indices(&indices);
                    let _ = vk.swapchain_loader.queue_present(vk.queue, &present_info);
                }
                frame = frame.wrapping_add(1);
            }
            _ => {}
        }
    });
}

/// Backward-compatible entry that uses empty layouts and a trivial vertex config.
/// Prefer `run_vulkan_linux_app_with<RB, VL>` for derived resource/vertex integration.
pub fn run_vulkan_linux_app(cfg: &EngineConfig) -> Result<(), VkError> {
    // Use a no-binding RB/VL adapter so code paths stay unified.
    struct NoRB; impl ResourceBindings for NoRB { fn bindings() -> &'static [crate::resources::BindingDesc] { &[] } }
    struct NoVL; impl VertexLayout for NoVL {
        fn vertex_attrs() -> &'static [crate::resources::VertexAttr] { &[] }
        fn vertex_buffers() -> &'static [crate::resources::VertexBufferDesc] {
            static BUFS: [crate::resources::VertexBufferDesc; 1] = [crate::resources::VertexBufferDesc { binding: 0, stride: 0, step: StepMode::Vertex }];
            &BUFS
        }
    }
    run_vulkan_linux_app_with::<NoRB, NoVL>(cfg)
}

/// Run using a derived RenderGraph: maps the first pass's attachments to a synthetic
/// PipelineDesc and delegates to `run_vulkan_linux_app_with`.
pub fn run_vulkan_linux_app_with_graph<RB, VL>(cfg: &EngineConfig, passes: &[&crate::render_graph::PassDesc]) -> Result<(), VkError>
where
    RB: ResourceBindings,
    VL: VertexLayout,
{
    use crate::pipeline::{PipelineDesc, ShaderPaths};
    if passes.is_empty() { return run_vulkan_linux_app_with::<RB, VL>(cfg); }
    let base = cfg.pipelines.first().ok_or_else(|| VkError::General("no base pipeline in EngineConfig".into()))?;
    let pass = passes[0];
    // Integrate resource planning based solely on pass descriptors
    let (resources, pass_plans) = plan_resources_from_passes(passes);
    let plan0 = &pass_plans[0];
    // Map planned resources to pipeline attachment descriptors for the first pass
    let mut ct_vec: Vec<crate::pipeline::ColorTargetDesc> = Vec::new();
    for &name in &plan0.colors {
        if let Some(r) = resources.iter().find(|r| r.name == name) {
            ct_vec.push(crate::pipeline::ColorTargetDesc { format: r.format, blend: None });
        }
    }
    let color_targets = if ct_vec.is_empty() { pass.color } else { let leaked: &'static [crate::pipeline::ColorTargetDesc] = Box::leak(ct_vec.into_boxed_slice()); Some(leaked) };
    let depth_target = if let Some(dname) = plan0.depth {
        if let Some(r) = resources.iter().find(|r| r.name == dname) {
            Some(crate::pipeline::DepthTargetDesc { format: r.format })
        } else { pass.depth.clone() }
    } else { pass.depth.clone() };
    let synth = PipelineDesc {
        name: "graph_pass_0",
        shaders: ShaderPaths { vs: base.shaders.vs, fs: base.shaders.fs },
        topology: base.topology.clone(),
        depth: base.depth,
        raster: base.raster.clone(),
        blend: base.blend.clone(),
        samples: base.samples.clone(),
        depth_stencil: base.depth_stencil.clone(),
        dynamic: base.dynamic.clone(),
        push_constants: base.push_constants.clone(),
        color_targets,
        depth_target,
    };
    let cfg2 = EngineConfig { app: cfg.app, window: cfg.window.clone(), pipelines: vec![synth], compute_pipelines: Vec::new(), options: cfg.options.clone() };
    run_vulkan_linux_app_with::<RB, VL>(&cfg2)
}

/// New entry with optional app-supplied demo resources.
pub fn run_vulkan_linux_app_with_resources<RB, VL>(cfg: &EngineConfig, resources: &AppResources) -> Result<(), VkError>
where
    RB: ResourceBindings,
    VL: VertexLayout,
{
    use winit::event::{Event, WindowEvent};
    use winit::event_loop::EventLoop;
    use winit::window::WindowBuilder;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(cfg.app)
        .with_inner_size(winit::dpi::LogicalSize::new(cfg.window.width as f64, cfg.window.height as f64))
        .build(&event_loop)
        .map_err(|e| VkError::General(format!("create window: {e}")))?;

    let mut vk = VkCore::new_with::<RB, VL>(&window, cfg, Some(resources))?;

    use crate::pipeline::Topology as MkTopology;
    for p in &cfg.pipelines {
        let topo_name = match p.topology { MkTopology::TriangleList => "TRIANGLE_LIST", MkTopology::LineList => "LINE_LIST", MkTopology::PointList => "POINT_LIST" };
        println!("[vk-linux] pipeline: '{}' topo={}", p.name, topo_name);
    }

    let mut frame: usize = 0;
    event_loop.run(move |event, _, control_flow| {
        use winit::event_loop::ControlFlow;
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => { *control_flow = ControlFlow::Exit; }
            Event::MainEventsCleared => {
                let i = frame % vk.images.len();
                unsafe {
                    let fence = vk.in_flight[i];
                    let image_avail = vk.image_available[i];
                    let render_fin = vk.render_finished[i];
                    let _ = vk.device.wait_for_fences(&[fence], true, u64::MAX);
                    let _ = vk.device.reset_fences(&[fence]);
                    let image_index = match vk.swapchain_loader.acquire_next_image(vk.swapchain, u64::MAX, image_avail, vk::Fence::null()) { Ok((idx, _)) => idx as usize, Err(_)=> { return; } };
                    let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
                    let submit_info = vk::SubmitInfo::builder()
                        .wait_semaphores(std::slice::from_ref(&image_avail))
                        .wait_dst_stage_mask(&wait_stages)
                        .command_buffers(std::slice::from_ref(&vk.command_buffers[image_index]))
                        .signal_semaphores(std::slice::from_ref(&render_fin));
                    let _ = vk.device.queue_submit(vk.queue, std::slice::from_ref(&submit_info), fence);
                    let indices = [image_index as u32];
                    let present_info = vk::PresentInfoKHR::builder().wait_semaphores(std::slice::from_ref(&render_fin)).swapchains(std::slice::from_ref(&vk.swapchain)).image_indices(&indices);
                    let _ = vk.swapchain_loader.queue_present(vk.queue, &present_info);
                }
                frame = frame.wrapping_add(1);
            }
            _ => {}
        }
    });
}

/// Entry with resources and a per-frame uniform update callback.
pub fn run_vulkan_linux_app_with_resources_and_update<RB, VL, F>(cfg: &EngineConfig, resources: &AppResources, mut update: F) -> Result<(), VkError>
where
    RB: ResourceBindings,
    VL: VertexLayout,
    F: 'static + FnMut(usize) -> Option<Vec<u8>>,
{
    use winit::event::{Event, WindowEvent};
    use winit::event_loop::EventLoop;
    use winit::window::WindowBuilder;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(cfg.app)
        .with_inner_size(winit::dpi::LogicalSize::new(cfg.window.width as f64, cfg.window.height as f64))
        .build(&event_loop)
        .map_err(|e| VkError::General(format!("create window: {e}")))?;

    let mut vk = VkCore::new_with::<RB, VL>(&window, cfg, Some(resources))?;

    let mut frame: usize = 0;
    event_loop.run(move |event, _, control_flow| {
        use winit::event_loop::ControlFlow;
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => { *control_flow = ControlFlow::Exit; }
            Event::MainEventsCleared => {
                let i = frame % vk.images.len();
                unsafe {
                    let fence = vk.in_flight[i];
                    let image_avail = vk.image_available[i];
                    let render_fin = vk.render_finished[i];
                    let _ = vk.device.wait_for_fences(&[fence], true, u64::MAX);
                    let _ = vk.device.reset_fences(&[fence]);
                    let image_index = match vk.swapchain_loader.acquire_next_image(vk.swapchain, u64::MAX, image_avail, vk::Fence::null()) { Ok((idx, _)) => idx as usize, Err(_)=> { return; } };

                    // Per-frame uniform update (size derived from VkCore::uniform_size_bytes)
                    if let Some(bytes) = update(image_index) {
                        if let Some(&mem) = vk.uniform_memories.get(image_index) {
                            let size = if vk.uniform_size_bytes == 0 { 64 } else { vk.uniform_size_bytes };
                            if let Ok(ptr) = vk.device.map_memory(mem, 0, size, vk::MemoryMapFlags::empty()) {
                                let n = bytes.len().min(size as usize);
                                std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, n);
                                if n < size as usize {
                                    // zero the rest
                                    std::ptr::write_bytes((ptr as *mut u8).add(n), 0u8, (size as usize) - n);
                                }
                                vk.device.unmap_memory(mem);
                            }
                        }
                    }

                    let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
                    let submit_info = vk::SubmitInfo::builder()
                        .wait_semaphores(std::slice::from_ref(&image_avail))
                        .wait_dst_stage_mask(&wait_stages)
                        .command_buffers(std::slice::from_ref(&vk.command_buffers[image_index]))
                        .signal_semaphores(std::slice::from_ref(&render_fin));
                    let _ = vk.device.queue_submit(vk.queue, std::slice::from_ref(&submit_info), fence);
                    let indices = [image_index as u32];
                    let present_info = vk::PresentInfoKHR::builder().wait_semaphores(std::slice::from_ref(&render_fin)).swapchains(std::slice::from_ref(&vk.swapchain)).image_indices(&indices);
                    let _ = vk.swapchain_loader.queue_present(vk.queue, &present_info);
                }
                frame = frame.wrapping_add(1);
            }
            _ => {}
        }
    });
}
