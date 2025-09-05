use gfx_dsl::vk_engine;
use gfx_dsl_support::{Engine, VulkanBackend};

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

fn main() {
    println!("App: {} ({}x{}, vsync={})",
        mgfx_cfg::CONFIG.app,
        mgfx_cfg::CONFIG.window.width,
        mgfx_cfg::CONFIG.window.height,
        mgfx_cfg::CONFIG.window.vsync,
    );

    // Create engine using Vulkan backend and initialize pipelines
    let engine = Engine::<VulkanBackend>::new_from_config(&mgfx_cfg::CONFIG);
    engine.init_pipelines(&mgfx_cfg::CONFIG);
    engine.frame();
}
