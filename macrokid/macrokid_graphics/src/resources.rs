#[derive(Clone, Debug)]
pub enum ResourceKind {
    Uniform,
    Texture,
    Sampler,
    CombinedImageSampler,
    // Extensions for compute/deferred paths
    StorageBuffer,
    StorageImage,
}

#[derive(Clone, Debug)]
pub struct BindingStages { pub vs: bool, pub fs: bool, pub cs: bool }

#[derive(Clone, Debug)]
pub struct BindingDesc {
    pub field: &'static str,
    pub set: u32,
    pub binding: u32,
    pub kind: ResourceKind,
    pub stages: Option<BindingStages>,
}

pub trait ResourceBindings { fn bindings() -> &'static [BindingDesc]; }

// Vertex layout types
#[derive(Clone, Debug)]
pub enum StepMode { Vertex, Instance }

#[derive(Clone, Debug)]
pub struct VertexBufferDesc { pub binding: u32, pub stride: u32, pub step: StepMode }

#[derive(Clone, Debug)]
pub struct VertexAttr { pub field: &'static str, pub binding: u32, pub location: u32, pub format: &'static str, pub offset: u32, pub size: u32 }

pub trait VertexLayout {
    fn vertex_attrs() -> &'static [VertexAttr];
    fn vertex_buffers() -> &'static [VertexBufferDesc];
}

// ============================================================================
// GPU Resource Tracking for Barrier Generation
// ============================================================================

#[cfg(feature = "vulkan-linux")]
use ash::vk;
use core::marker::PhantomData;
use core::any::TypeId;

#[cfg(feature = "vulkan-linux")]
use core::sync::atomic::{AtomicU32, Ordering};

/// Metadata about a GPU resource for barrier code generation.
/// Used by derive macros to track resource access patterns.
#[derive(Clone)]
pub struct GpuResourceMeta {
    pub type_id: TypeId,
    pub type_name: &'static str,
    pub resource_kind: GpuResourceKind,
    #[cfg(feature = "vulkan-linux")]
    pub read_stage: vk::PipelineStageFlags,
    #[cfg(feature = "vulkan-linux")]
    pub write_stage: vk::PipelineStageFlags,
    #[cfg(feature = "vulkan-linux")]
    pub read_access: vk::AccessFlags,
    #[cfg(feature = "vulkan-linux")]
    pub write_access: vk::AccessFlags,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuResourceKind {
    Buffer,
    Image,
}

/// Trait for GPU-accessible resources that participate in barrier generation.
///
/// Types implementing this trait provide metadata about their Vulkan synchronization
/// requirements, enabling automatic barrier hint generation via derive macros.
#[cfg(feature = "vulkan-linux")]
pub trait GpuResource: Send + Sync + 'static {
    /// Get the Vulkan resource handle for barrier construction
    fn handle(&self) -> GpuHandle;

    /// Default pipeline stage for read operations
    fn read_stage() -> vk::PipelineStageFlags;

    /// Default pipeline stage for write operations
    fn write_stage() -> vk::PipelineStageFlags;

    /// Default access mask for read operations
    fn read_access() -> vk::AccessFlags;

    /// Default access mask for write operations
    fn write_access() -> vk::AccessFlags;

    /// Resource kind (Buffer or Image)
    fn resource_kind() -> GpuResourceKind;

    /// Generate metadata for derive macro consumption
    fn metadata() -> GpuResourceMeta {
        GpuResourceMeta {
            type_id: TypeId::of::<Self>(),
            type_name: core::any::type_name::<Self>(),
            resource_kind: Self::resource_kind(),
            read_stage: Self::read_stage(),
            write_stage: Self::write_stage(),
            read_access: Self::read_access(),
            write_access: Self::write_access(),
        }
    }
}

/// Type-erased GPU resource handle for barrier construction
#[cfg(feature = "vulkan-linux")]
#[derive(Clone, Copy)]
pub enum GpuHandle {
    Buffer(vk::Buffer),
    Image(vk::Image, ImageLayoutInfo),
}

/// Image layout transition information
#[cfg(feature = "vulkan-linux")]
#[derive(Clone, Copy)]
pub struct ImageLayoutInfo {
    pub current_layout: vk::ImageLayout,
    pub target_layout: vk::ImageLayout,
}

/// GPU buffer wrapper with type-level synchronization hints
#[cfg(feature = "vulkan-linux")]
pub struct GpuBuffer<T: Send + Sync> {
    pub buffer: vk::Buffer,
    pub size: u64,
    _phantom: PhantomData<T>,
}

#[cfg(feature = "vulkan-linux")]
impl<T: Send + Sync + 'static> GpuBuffer<T> {
    pub fn new(buffer: vk::Buffer, size: u64) -> Self {
        Self { buffer, size, _phantom: PhantomData }
    }

    /// Infer pipeline stage from type name heuristics
    fn infer_read_stage() -> vk::PipelineStageFlags {
        let type_name = core::any::type_name::<T>();

        if type_name.contains("Vertex") || type_name.contains("Index") {
            vk::PipelineStageFlags::VERTEX_INPUT
        } else if type_name.contains("Uniform") || type_name.contains("Ubo") {
            vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER
        } else if type_name.contains("Storage") {
            vk::PipelineStageFlags::COMPUTE_SHADER
        } else {
            // Conservative default: assume all shader stages
            vk::PipelineStageFlags::ALL_COMMANDS
        }
    }

    /// Infer access mask from type name heuristics
    fn infer_read_access() -> vk::AccessFlags {
        let type_name = core::any::type_name::<T>();

        if type_name.contains("Vertex") {
            vk::AccessFlags::VERTEX_ATTRIBUTE_READ
        } else if type_name.contains("Index") {
            vk::AccessFlags::INDEX_READ
        } else if type_name.contains("Uniform") || type_name.contains("Ubo") {
            vk::AccessFlags::UNIFORM_READ
        } else if type_name.contains("Storage") {
            vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE
        } else {
            vk::AccessFlags::SHADER_READ
        }
    }
}

#[cfg(feature = "vulkan-linux")]
impl<T: Send + Sync + 'static> GpuResource for GpuBuffer<T> {
    fn handle(&self) -> GpuHandle {
        GpuHandle::Buffer(self.buffer)
    }

    fn read_stage() -> vk::PipelineStageFlags {
        Self::infer_read_stage()
    }

    fn write_stage() -> vk::PipelineStageFlags {
        // Most writes to buffers are transfers (uploads)
        vk::PipelineStageFlags::TRANSFER
    }

    fn read_access() -> vk::AccessFlags {
        Self::infer_read_access()
    }

    fn write_access() -> vk::AccessFlags {
        vk::AccessFlags::TRANSFER_WRITE
    }

    fn resource_kind() -> GpuResourceKind {
        GpuResourceKind::Buffer
    }
}

/// GPU image wrapper with layout tracking and type-level hints
#[cfg(feature = "vulkan-linux")]
pub struct GpuImage<T: Send + Sync> {
    pub image: vk::Image,
    pub extent: vk::Extent2D,
    pub format: vk::Format,
    current_layout: AtomicU32,  // Stores vk::ImageLayout as u32
    _phantom: PhantomData<T>,
}

#[cfg(feature = "vulkan-linux")]
impl<T: Send + Sync + 'static> GpuImage<T> {
    pub fn new(image: vk::Image, extent: vk::Extent2D, format: vk::Format) -> Self {
        Self {
            image,
            extent,
            format,
            current_layout: AtomicU32::new(vk::ImageLayout::UNDEFINED.as_raw() as u32),
            _phantom: PhantomData,
        }
    }

    pub fn with_layout(image: vk::Image, extent: vk::Extent2D, format: vk::Format, layout: vk::ImageLayout) -> Self {
        Self {
            image,
            extent,
            format,
            current_layout: AtomicU32::new(layout.as_raw() as u32),
            _phantom: PhantomData,
        }
    }

    pub fn current_layout(&self) -> vk::ImageLayout {
        vk::ImageLayout::from_raw(self.current_layout.load(Ordering::Acquire) as i32)
    }

    pub fn set_layout(&self, layout: vk::ImageLayout) {
        self.current_layout.store(layout.as_raw() as u32, Ordering::Release);
    }

    /// Infer target layout from type name heuristics
    fn infer_target_layout() -> vk::ImageLayout {
        let type_name = core::any::type_name::<T>();

        if type_name.contains("RenderTarget") || type_name.contains("ColorAttachment") {
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        } else if type_name.contains("DepthStencil") || type_name.contains("Depth") {
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
        } else if type_name.contains("Texture") || type_name.contains("Sampled") {
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        } else if type_name.contains("Storage") {
            vk::ImageLayout::GENERAL
        } else {
            vk::ImageLayout::GENERAL
        }
    }

    /// Infer read stage from type name
    fn infer_read_stage() -> vk::PipelineStageFlags {
        let type_name = core::any::type_name::<T>();

        if type_name.contains("Texture") || type_name.contains("Sampled") {
            vk::PipelineStageFlags::FRAGMENT_SHADER
        } else if type_name.contains("Storage") {
            vk::PipelineStageFlags::COMPUTE_SHADER
        } else {
            vk::PipelineStageFlags::FRAGMENT_SHADER
        }
    }

    /// Infer write stage from type name
    fn infer_write_stage() -> vk::PipelineStageFlags {
        let type_name = core::any::type_name::<T>();

        if type_name.contains("RenderTarget") || type_name.contains("ColorAttachment") {
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
        } else if type_name.contains("DepthStencil") || type_name.contains("Depth") {
            vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS
        } else if type_name.contains("Storage") {
            vk::PipelineStageFlags::COMPUTE_SHADER
        } else {
            vk::PipelineStageFlags::TRANSFER
        }
    }
}

#[cfg(feature = "vulkan-linux")]
impl<T: Send + Sync + 'static> GpuResource for GpuImage<T> {
    fn handle(&self) -> GpuHandle {
        GpuHandle::Image(self.image, ImageLayoutInfo {
            current_layout: self.current_layout(),
            target_layout: Self::infer_target_layout(),
        })
    }

    fn read_stage() -> vk::PipelineStageFlags {
        Self::infer_read_stage()
    }

    fn write_stage() -> vk::PipelineStageFlags {
        Self::infer_write_stage()
    }

    fn read_access() -> vk::AccessFlags {
        let type_name = core::any::type_name::<T>();

        if type_name.contains("Texture") || type_name.contains("Sampled") {
            vk::AccessFlags::SHADER_READ
        } else if type_name.contains("Storage") {
            vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE
        } else {
            vk::AccessFlags::SHADER_READ
        }
    }

    fn write_access() -> vk::AccessFlags {
        let type_name = core::any::type_name::<T>();

        if type_name.contains("RenderTarget") || type_name.contains("ColorAttachment") {
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE
        } else if type_name.contains("DepthStencil") || type_name.contains("Depth") {
            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE
        } else if type_name.contains("Storage") {
            vk::AccessFlags::SHADER_WRITE
        } else {
            vk::AccessFlags::TRANSFER_WRITE
        }
    }

    fn resource_kind() -> GpuResourceKind {
        GpuResourceKind::Image
    }
}

// ============================================================================
// GPU Resource Access Trait (extends macrokid_core::threads::ResourceAccess)
// ============================================================================

/// Extension of ResourceAccess for GPU-aware systems.
///
/// This trait provides GPU synchronization metadata for barrier generation.
/// It lives alongside CPU ResourceAccess and is implemented automatically
/// by #[derive(System)] when GPU resources are detected.
pub trait GpuResourceAccess {
    /// GPU resources that this system reads from
    fn gpu_reads() -> &'static [GpuResourceMeta] { &[] }

    /// GPU resources that this system writes to
    fn gpu_writes() -> &'static [GpuResourceMeta] { &[] }

    /// Generate barrier requirement hints for this system
    fn barrier_requirements() -> String {
        let mut hints = String::from("GPU Barrier Requirements:\n\n");

        if Self::gpu_reads().is_empty() && Self::gpu_writes().is_empty() {
            hints.push_str("  (no GPU resources accessed)\n");
            return hints;
        }

        for meta in Self::gpu_reads() {
            hints.push_str(&format!(
                "  READ: {}\n\
                     Resource: {:?}\n",
                meta.type_name,
                meta.resource_kind
            ));

            #[cfg(feature = "vulkan-linux")]
            {
                hints.push_str(&format!(
                    "    Stage: {:?} → {:?}\n\
                         Access: {:?} → {:?}\n\
                         Action: Insert buffer/image barrier before read\n\n",
                    meta.write_stage,
                    meta.read_stage,
                    meta.write_access,
                    meta.read_access
                ));
            }
        }

        for meta in Self::gpu_writes() {
            hints.push_str(&format!(
                "  WRITE: {}\n\
                     Resource: {:?}\n",
                meta.type_name,
                meta.resource_kind
            ));

            #[cfg(feature = "vulkan-linux")]
            {
                hints.push_str(&format!(
                    "    Stage: {:?}\n\
                         Access: {:?}\n",
                    meta.write_stage,
                    meta.write_access
                ));

                if meta.resource_kind == GpuResourceKind::Image {
                    hints.push_str("    Layout transition may be required\n");
                }
                hints.push_str("\n");
            }
        }

        hints
    }
}
