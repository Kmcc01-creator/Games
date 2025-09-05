# Graphics DSL Development Roadmap

## Architecture & Performance Strategy

## Current Implementation Analysis

### Overview
The current `gfx_dsl` example implements a Vulkan engine configuration DSL using a single large `vk_engine!` macro that parses complex nested configuration syntax.

### Current Architecture
- **Macro**: `vk_engine!` - Single monolithic procedural macro
- **Parser**: ~113 lines of manual `Parse` implementations
- **AST Types**: 5 separate AST structs for different config sections
- **Codegen**: Direct token stream generation to final engine code

### Current Usage Pattern
```rust
vk_engine! {
    {
        app: "MacroKid Vulkan Demo",
        window: { width: 1024, height: 600, vsync: true },
        graph: {
            pass main {
                pipelines: [
                    pipeline triangle {
                        vs: "shaders/triangle.vert",
                        fs: "shaders/triangle.frag",
                        topology: TriangleList,
                        depth: false,
                    },
                    pipeline lines {
                        vs: "shaders/lines.vert", 
                        fs: "shaders/lines.frag",
                        topology: LineList,
                        depth: false,
                    }
                ]
            }
        }
    }
}
```

## Problems with Current Approach

### 1. **Complex Manual Parsing**
- Manual token stream parsing with string-based key matching
- Nested brace parsing logic scattered across multiple `impl Parse`
- Error-prone and difficult to extend with new configuration options

### 2. **Poor Error Messages**
- Generic parsing errors that don't provide clear guidance
- No compile-time validation of configuration relationships
- Runtime string matching for configuration keys

### 3. **Monolithic Design**
- Single large macro handles all concerns (window, graphics, pipelines)
- Adding new features requires modifying the core parser
- Difficult to reuse components across different engine configurations

### 4. **Limited Composability**
- Cannot easily share pipeline configurations between different engines
- No way to programmatically build configurations
- Static-only configuration - no runtime flexibility

### 5. **Maintenance Overhead**
- ~300 lines of complex procedural macro code
- Interleaved parsing, validation, and codegen logic
- Difficult to test individual components

## Proposed Solution: Builder Pattern Architecture

### Core Philosophy
Replace the monolithic macro approach with a composable builder pattern that leverages Rust's type system for validation and provides both compile-time and runtime configuration flexibility.

### Builder Pattern Benefits for Vulkan
Vulkan configurations are inherently complex with:
- Multiple interdependent components (devices, pipelines, render passes)
- Optional and conditional settings
- Need for validation at multiple stages
- Complex resource relationships

The builder pattern naturally handles this complexity through:
- **Incremental Construction**: Build configurations step-by-step
- **Type-State Patterns**: Encode validation in the type system
- **Fluent APIs**: Clear, readable configuration code
- **Composability**: Reuse and combine configuration components

## Proposed Architecture

### 1. **Core Builder Types**
```rust
// Main engine builder
#[derive(VkEngineBuilder)]
pub struct EngineConfigBuilder<State = Empty> {
    state: State,
    config: EngineConfig,
}

// Type states for compile-time validation
pub struct Empty;
pub struct HasApp;
pub struct HasWindow; 
pub struct HasGraph;
pub struct Complete;

// Configuration data structures
pub struct EngineConfig {
    pub app: Option<String>,
    pub window: Option<WindowConfig>,
    pub graph: Option<RenderGraph>,
}

pub struct WindowConfig {
    pub width: u32,
    pub height: u32, 
    pub vsync: bool,
}

pub struct RenderGraph {
    pub passes: Vec<RenderPass>,
}

pub struct RenderPass {
    pub name: String,
    pub pipelines: Vec<Pipeline>,
}

pub struct Pipeline {
    pub name: String,
    pub shaders: ShaderConfig,
    pub topology: Topology,
    pub depth: bool,
}
```

### 2. **Builder Implementation**
```rust
impl EngineConfigBuilder<Empty> {
    pub fn new() -> Self {
        Self {
            state: Empty,
            config: EngineConfig::default(),
        }
    }
    
    pub fn app(self, name: &str) -> EngineConfigBuilder<HasApp> {
        EngineConfigBuilder {
            state: HasApp,
            config: EngineConfig {
                app: Some(name.to_string()),
                ..self.config
            },
        }
    }
}

impl EngineConfigBuilder<HasApp> {
    pub fn window(self, width: u32, height: u32, vsync: bool) -> EngineConfigBuilder<HasWindow> {
        EngineConfigBuilder {
            state: HasWindow,
            config: EngineConfig {
                window: Some(WindowConfig { width, height, vsync }),
                ..self.config
            },
        }
    }
}

impl EngineConfigBuilder<HasWindow> {
    pub fn graph(self) -> GraphBuilder<Self> {
        GraphBuilder::new(self)
    }
}

// Nested builder for render graph construction
pub struct GraphBuilder<Parent> {
    parent: Parent,
    graph: RenderGraph,
}

impl<Parent> GraphBuilder<Parent> {
    pub fn add_pass(mut self, name: &str) -> PassBuilder<Self> {
        PassBuilder::new(self, name)
    }
    
    pub fn finish(self) -> Parent::WithGraph {
        // Return parent with completed graph
    }
}

pub struct PassBuilder<Parent> {
    parent: Parent,
    pass: RenderPass,
}

impl<Parent> PassBuilder<Parent> {
    pub fn add_pipeline(mut self, name: &str) -> PipelineBuilder<Self> {
        PipelineBuilder::new(self, name)
    }
    
    pub fn finish_pass(mut self) -> Parent {
        self.parent.add_pass(self.pass);
        self.parent
    }
}

pub struct PipelineBuilder<Parent> {
    parent: Parent,
    pipeline: Pipeline,
}

impl<Parent> PipelineBuilder<Parent> {
    pub fn shaders(mut self, vs: &str, fs: &str) -> Self {
        self.pipeline.shaders = ShaderConfig::new(vs, fs);
        self
    }
    
    pub fn topology(mut self, topology: Topology) -> Self {
        self.pipeline.topology = topology;
        self
    }
    
    pub fn depth(mut self, enabled: bool) -> Self {
        self.pipeline.depth = enabled;
        self
    }
    
    pub fn finish_pipeline(mut self) -> Parent {
        self.parent.add_pipeline(self.pipeline);
        self.parent
    }
}

impl EngineConfigBuilder<Complete> {
    pub fn build(self) -> Engine {
        Engine::from_config(self.config)
    }
}
```

### 3. **Usage Example**
```rust
fn main() {
    let engine = EngineConfigBuilder::new()
        .app("MacroKid Vulkan Demo")
        .window(1024, 600, true)
        .graph()
            .add_pass("main")
                .add_pipeline("triangle")
                    .shaders("shaders/triangle.vert", "shaders/triangle.frag")
                    .topology(Topology::TriangleList)
                    .depth(false)
                    .finish_pipeline()
                .add_pipeline("lines")
                    .shaders("shaders/lines.vert", "shaders/lines.frag") 
                    .topology(Topology::LineList)
                    .depth(false)
                    .finish_pipeline()
                .finish_pass()
            .finish()
        .build();
        
    engine.init_pipelines();
    engine.frame();
}
```

## Benefits of Builder Approach

### 1. **Type Safety**
- Compile-time enforcement of required configuration steps
- Type states prevent invalid configurations (e.g., building without required app name)
- Method chaining provides natural validation flow

### 2. **Composability** 
- Individual builders can be extracted and reused
- Configuration components can be shared between different engines
- Programmatic configuration construction enables dynamic setups

### 3. **Maintainability**
- Clear separation of concerns (each builder handles one concept)
- Easy to extend with new configuration options
- Each component can be tested independently

### 4. **Flexibility**
- Both compile-time and runtime configuration possible
- Conditional configuration based on runtime conditions
- Easy to serialize/deserialize configurations

### 5. **Error Handling**
- Builder methods can return `Result` types for validation
- Clear error messages at the point of configuration
- Fail-fast behavior prevents invalid engine creation

## Implementation Strategy

### Phase 1: Core Builder Infrastructure
1. Define base builder types and configuration structs
2. Implement basic `EngineConfigBuilder` with type states
3. Create derive macro `#[derive(VkEngineBuilder)]` for codegen

### Phase 2: Nested Builder Support  
1. Implement `GraphBuilder`, `PassBuilder`, `PipelineBuilder`
2. Add builder composition and parent-child relationships
3. Ensure proper type state transitions

### Phase 3: Advanced Features
1. Add validation and error handling
2. Implement configuration serialization/deserialization  
3. Add runtime configuration modification support

### Phase 4: Migration and Testing
1. Create compatibility layer with existing `vk_engine!` macro
2. Port existing examples to builder pattern
3. Add comprehensive test suite

## Code Metrics Comparison

| Metric | Current Implementation | Proposed Builder |
|--------|----------------------|------------------|
| Lines of Code (macro) | ~300 | ~150 |
| Parse Implementation | Manual, ~113 lines | Derive-generated |
| Error Handling | Basic parsing errors | Type-safe + validation |
| Extensibility | Modify core parser | Add new builder methods |
| Testability | Integration only | Unit testable components |
| Reusability | None | High - composable builders |

## Conclusion

The builder pattern approach provides a more maintainable, flexible, and type-safe alternative to the current macro-based DSL while maintaining the same level of expressiveness. The incremental construction model naturally fits Vulkan's complex configuration requirements and provides room for future enhancements without breaking existing code.

The key insight is that complex configurations benefit from **progressive construction** rather than **declarative specification**, especially when validation and error handling are critical concerns.

---

# Data-Oriented Design Strategy

## Philosophy: Performance Through Data Layout

While the current focus is on API design and usability, we must plan for eventual performance optimization through **Data-Oriented Design (DoD)**. This approach separates data by access patterns rather than logical grouping.

## Why Not "Make Everything Fast"?

### Memory Hierarchy Reality
- **L1 cache**: ~32KB per core (ultra-fast)
- **L2 cache**: ~256KB per core (fast) 
- **L3 cache**: ~8MB shared (moderate)
- **RAM**: Unlimited but slow

Making everything "hot" actually **slows down truly hot data** by cache pollution.

### Access Pattern Categories

#### Ultra-Hot Data (accessed every frame, every object)
```rust
// Needs SIMD alignment and tight packing
mesh_ids: Vec<u32>,           // 4KB for 1000 objects
transform_indices: Vec<u32>,  // 4KB for 1000 objects  
material_indices: Vec<u32>,   // 4KB for 1000 objects
// Total: 12KB - fits in L1 cache
```

#### Warm Data (accessed on state changes)
```rust  
// Accessed when pipelines/materials change
pipeline_configs: Vec<PipelineDesc>,  // ~50 configs max
shader_bindings: Vec<BindingInfo>,    // Setup once per pass
topology_states: Vec<Topology>,       // Changes infrequently
```

#### Cold Data (debug/tools/rare events)
```rust
// Only in debug builds or development tools
debug_names: Vec<String>,        // Expensive strings
creation_timestamps: Vec<u64>,   // Profiling only
source_locations: Vec<String>,   // Asset debugging
validation_metadata: Vec<Metadata>, // Debug mode only
```

## Staged Implementation Strategy

### Phase 1: API Stabilization (Current Focus)
**Priority**: Get the DSL and builder patterns right
- Keep simple object-oriented structures
- Focus on usability and correctness
- Design interfaces that will work with both AoS and SoA

```rust
// Current approach - simple and debuggable
struct Pipeline {
    name: String,        // Will become "cold" later
    topology: Topology,  // Will become "hot" later
    depth: bool,         // Will become "hot" later
    shaders: ShaderPaths, // Will become "warm" later
}
```

### Phase 2: Usage Pattern Analysis (Future)
**Trigger**: When you have real applications using the DSL
- Add access pattern profiling to existing structures
- Gather data on which fields are accessed when
- Identify actual performance bottlenecks

```rust
#[derive(ProfileAccess)]  // Future macrokid capability
struct Pipeline {
    #[access_tracking(heat = "cold")]
    name: String,
    #[access_tracking(heat = "hot")] 
    topology: Topology,
    // Instrumentation gathers real usage data
}
```

### Phase 3: DoD Transformation (Much Later)
**Trigger**: Actual performance issues with large datasets (1000+ objects)
- Transform based on real profiling data, not assumptions
- Use macrokid to generate both AoS and SoA views automatically
- Maintain API compatibility through generated adapters

```rust
#[derive(DataOriented)]
#[profile_data("pipeline_access.json")]  // From Phase 2
pub struct PipelineData {
    #[heat(ultra)] topology: Topology,     // Packed together
    #[heat(ultra)] depth: bool,            // SIMD-aligned  
    #[heat(warm)] shaders: ShaderPaths,    // Separate allocation
    #[heat(cold)] name: String,            // Debug-only memory
}

// Generated by macrokid:
pub struct PipelineDataSoA {
    // Ultra-hot: single cache line, SIMD-friendly
    pub topologies: Vec<Topology>,
    pub depths: Vec<bool>,
    
    // Warm: separate allocation, loaded on demand  
    pub shaders: Vec<ShaderPaths>,
    
    // Cold: heap-allocated, debug builds only
    pub names: Vec<String>,
}
```

## DoD Design Principles for Future Implementation

### 1. Heat-Based Separation
```rust
#[derive(DataOriented)]
pub struct RenderCommand {
    // Ultra-hot: accessed in tight loops
    #[heat(ultra, simd_align)] mesh_id: u32,
    #[heat(ultra, simd_align)] material_id: u32,
    
    // Hot: accessed per rendering pass
    #[heat(hot)] depth: f32,
    #[heat(hot)] sort_key: u64,
    
    // Warm: setup/validation
    #[heat(warm)] pipeline_id: u32,
    
    // Cold: debug/development only
    #[heat(cold)] debug_name: String,
}
```

### 2. Component Storage Patterns
```rust
#[derive(ComponentStorage)]
pub struct Transform {
    #[simd_aligned] position: Vec3,  // 16-byte aligned
    #[simd_aligned] rotation: Quat,  // 16-byte aligned
    #[simd_aligned] scale: Vec3,     // 16-byte aligned
}

#[derive(ComponentStorage)]
pub struct Material {
    #[packed] diffuse: ColorId,      // Tight packing
    #[packed] roughness: f32,        // Cache-friendly
    #[packed] metallic: f32,         // Minimize memory
}
```

### 3. Batch Processing Generation
```rust
#[derive(BatchProcessor)]
pub struct RenderProcessor {
    #[batch_size(64)]    // Process in chunks
    topology_updates: TopologyBatch,
    
    #[simd]              // Auto-vectorization
    depth_calculations: DepthBatch,
    
    #[prefetch]          // Hardware prefetching hints
    material_loads: MaterialBatch,
}
```

## Implementation Timeline

### Current Phase (API Focus)
- ✅ Builder pattern implementation
- ✅ Better error messages with macrokid diagnostics  
- 🔄 Macro composition and reusability
- ⏳ Multi-backend support architecture

### Future Phase (Performance Focus) 
- ⏳ Access pattern profiling infrastructure
- ⏳ Heat-based data layout generation
- ⏳ SIMD batch processing code generation
- ⏳ Memory pool management for different heat levels

## Why This Approach Works

### Early Development Benefits
- **Simple debugging** - clear object relationships
- **Fast iteration** - no complex generated code to understand
- **Clear interfaces** - familiar patterns for users
- **API flexibility** - easy to change without performance concerns

### Later Optimization Benefits  
- **Data-driven decisions** - optimize based on real usage, not guesses
- **Mature tooling** - macrokid infrastructure will be battle-tested
- **Proven bottlenecks** - know which optimizations actually matter
- **Backward compatibility** - existing code continues to work

The key insight: **API design and performance optimization are separate concerns**. Get the API right first, then optimize based on real usage patterns.