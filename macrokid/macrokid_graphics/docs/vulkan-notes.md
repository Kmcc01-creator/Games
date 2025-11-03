# Vulkan Backend Notes

## Platform Support

Currently supported:
- **Linux** - Full Vulkan support via `vulkan-linux` feature

Planned:
- Windows (D3D12/Vulkan)
- macOS (MoltenVK)
- Wayland compositor integration

## Requirements

### Linux

**Install Vulkan SDK:**
```bash
# Ubuntu/Debian
sudo apt install vulkan-tools libvulkan-dev vulkan-validationlayers

# Arch
sudo pacman -S vulkan-tools vulkan-headers vulkan-validation-layers

# Fedora
sudo dnf install vulkan-tools vulkan-headers vulkan-validation-layers
```

**Verify installation:**
```bash
vulkaninfo
```

**Graphics drivers:**
- NVIDIA: proprietary driver (nvidia-driver)
- AMD: Mesa (RADV)
- Intel: Mesa (ANV)

## Feature Flags

### vulkan-linux
Enables the Linux Vulkan backend.

```toml
macrokid_graphics = { path = "../macrokid_graphics", features = ["vulkan-linux"] }
```

### vk-shaderc-compile
Enables runtime GLSL shader compilation using shaderc.

```toml
macrokid_graphics = { path = "../macrokid_graphics", features = ["vk-shaderc-compile"] }
```

Without this, you must provide pre-compiled SPIR-V shaders.

### proto
Enables protobuf-based configuration loading.

```toml
macrokid_graphics = { path = "../macrokid_graphics", features = ["proto"] }
```

## Vulkan Objects

The framework manages these Vulkan objects automatically:

- **Instance** - Created from app config
- **Device** - Selected based on available GPUs
- **Swapchain** - Configured from window settings
- **Descriptor Sets** - Generated from `ResourceBinding` derives
- **Pipeline Layouts** - Inferred from resource bindings
- **Graphics Pipelines** - Built from `GraphicsPipeline` derives
- **Vertex Input State** - Generated from `BufferLayout` derives

## Validation Layers

Validation layers are enabled in debug builds automatically.

**Environment variables:**
```bash
# Enable all validation
export VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation

# Verbose output
export VK_LOADER_DEBUG=all
```

**Debugging tips:**
- Check validation layer messages for API misuse
- Use `vulkaninfo` to verify device capabilities
- Use RenderDoc for frame capture and analysis

## Shader Compilation

### Runtime Compilation (vk-shaderc-compile)

Shaders are compiled at runtime from GLSL source:

```rust
#[derive(GraphicsPipeline)]
#[vertex_shader("shaders/my.vert")]
#[fragment_shader("shaders/my.frag")]
struct MyPipeline { /* ... */ }
```

**Pros:**
- Fast iteration (edit shaders, rerun)
- No build step required

**Cons:**
- Startup time overhead
- Requires shaderc library

### Pre-compiled SPIR-V

Compile shaders ahead of time:

```bash
glslc shaders/my.vert -o shaders/my.vert.spv
glslc shaders/my.frag -o shaders/my.frag.spv
```

Update derives:
```rust
#[vertex_shader("shaders/my.vert.spv")]
#[fragment_shader("shaders/my.frag.spv")]
```

**Pros:**
- Faster startup
- No shaderc dependency

**Cons:**
- Extra build step
- Less convenient during development

## Resource Limits

Common Vulkan limits to be aware of:

- **Max descriptor sets**: 4 (typical)
- **Max bindings per set**: 16-32 (varies by device)
- **Max uniform buffer size**: 16-64KB (varies)
- **Max storage buffer size**: Device-dependent

Query limits at runtime via `vkGetPhysicalDeviceProperties`.

## Memory Management

The framework handles:
- Buffer allocation (vertex, index, uniform)
- Image allocation (textures)
- Memory binding
- Synchronization (fences, semaphores)

**Best practices:**
- Reuse buffers when possible
- Batch updates to reduce memory copies
- Use staging buffers for large transfers

## Synchronization

Implicit synchronization is handled for:
- Swapchain image acquisition
- Command buffer submission
- Presentation

**Manual synchronization needed for:**
- Multi-threaded command recording (future)
- Compute/graphics queue coordination (future)

## Performance Tips

1. **Minimize state changes** - Group draws by pipeline
2. **Use instancing** - Draw many objects with one call
3. **Persistent mapping** - Map uniform buffers once
4. **Descriptor pooling** - Reuse descriptor sets
5. **Pipeline caching** - Cache compiled pipelines (future)

## Debugging

### Common Issues

**"Failed to create instance":**
- Check Vulkan installation
- Verify driver support
- Try `vulkaninfo` to diagnose

**"No suitable GPU found":**
- Update graphics drivers
- Check GPU supports Vulkan 1.0+
- Verify device is not disabled

**"Shader compilation failed":**
- Check GLSL syntax
- Verify shader stage matches attribute (`#[vertex_shader]` vs `#[fragment_shader]`)
- Look for missing `#version` directive

**"Descriptor binding mismatch":**
- Ensure shader bindings match `ResourceBinding` attributes
- Check set/binding numbers
- Verify resource types (uniform vs storage vs texture)

### Tools

- **vulkaninfo** - Check Vulkan installation and device capabilities
- **RenderDoc** - Frame capture and GPU debugging
- **Nsight Graphics** - NVIDIA GPU profiler
- **Radeon GPU Profiler** - AMD GPU profiler

## Extensions

Currently used extensions:
- `VK_KHR_surface` - Window surface creation
- `VK_KHR_swapchain` - Presentation
- Platform-specific surface extensions (e.g., `VK_KHR_xlib_surface` on Linux X11)

Future extensions:
- `VK_KHR_ray_tracing` - Ray tracing support
- `VK_EXT_descriptor_indexing` - Bindless resources
- `VK_KHR_dynamic_rendering` - Modern render pass API

## Known Limitations

- Single queue family (graphics/present)
- No multi-threading (yet)
- No compute shaders (yet)
- No geometry/tessellation shaders (yet)
- Fixed render pass structure

These will be addressed in future releases.

## Further Reading

- [Vulkan Specification](https://www.khronos.org/registry/vulkan/specs/1.3-extensions/html/)
- [Vulkan Tutorial](https://vulkan-tutorial.com/)
- [Vulkan Guide](https://github.com/KhronosGroup/Vulkan-Guide)
- [vkguide.dev](https://vkguide.dev/)
