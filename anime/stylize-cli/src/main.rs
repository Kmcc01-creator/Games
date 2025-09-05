use anyhow::Result;
use clap::{Parser, Subcommand};
use stylize_core::{asset_dna, VERSION};

#[derive(Parser, Debug)]
#[command(name = "stylize", version = VERSION, about = "Procedural anime asset tools")] 
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Load and inspect an Asset DNA YAML
    Inspect { path: String },
    /// List Vulkan devices (requires --features vulkan)
    #[cfg(feature = "vulkan")]
    VkInfo,
    /// Create/destroy shader modules to verify SPIR-V embedding
    #[cfg(feature = "vulkan")]
    VkTestShaders,
    /// Render a simple offscreen image and write PNG
    #[cfg(feature = "vulkan")]
    VkRenderTest {
        #[arg(long, default_value_t = 512)]
        width: u32,
        #[arg(long, default_value_t = 512)]
        height: u32,
        #[arg(long, default_value = "out.png")]
        out: String,
    },
    /// Render a G-buffer and write albedo/normal PNGs
    #[cfg(feature = "vulkan")]
    VkGbufferTest {
        #[arg(long, default_value_t = 512)]
        width: u32,
        #[arg(long, default_value_t = 512)]
        height: u32,
        #[arg(long, default_value = "gbuf")] 
        out_prefix: String,
    },
    /// Render toon from G-buffer and save PNG
    #[cfg(feature = "vulkan")]
    VkToonFromGbuf {
        #[arg(long, default_value_t = 512)]
        width: u32,
        #[arg(long, default_value_t = 512)]
        height: u32,
        #[arg(long, default_value = "toon.png")]
        out: String,
    },
    /// Render UV-sphere mesh into G-buffer and save albedo/normal
    #[cfg(feature = "vulkan")]
    VkGbufferMesh {
        #[arg(long, default_value_t = 512)]
        width: u32,
        #[arg(long, default_value_t = 512)]
        height: u32,
        #[arg(long, default_value = "mesh")] 
        out_prefix: String,
    },
    /// Render toon shading from mesh G-buffer and save PNG
    #[cfg(feature = "vulkan")]
    VkToonMesh {
        #[arg(long, default_value_t = 512)]
        width: u32,
        #[arg(long, default_value_t = 512)]
        height: u32,
        #[arg(long, default_value = "toon-mesh.png")]
        out: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Command::Inspect { path } => {
            let dna = asset_dna::load_from_path(&path)?;
            println!("Loaded DNA: {}", dna.id);
            println!("  head_scale: {}", dna.proportions.head_scale);
            println!("  eye_scale: {}", dna.proportions.eye_scale);
            println!("  hair: {} (strands={}, k={:.2}, d={:.2})", dna.hair.style, dna.hair.strands, dna.hair.stiffness, dna.hair.damping);
            println!("  clothes: {} (folds={})", dna.clothes.top, dna.clothes.skirt_folds);
            println!("  shading bands: {}", dna.shading.bands);
            println!("  shadow thresholds: face={:.2}, cloth={:.2}", dna.shading.face_shadow_threshold, dna.shading.cloth_shadow_threshold);
            println!("  line width: {:.2}px, crease angle: {:.0}°", dna.lines.width_px, dna.lines.crease_angle_deg);
        }
        #[cfg(feature = "vulkan")]
        Command::VkInfo => {
            let list = stylize_core::render::vk::enumerate_devices()?;
            if list.is_empty() { println!("No Vulkan devices found"); }
            for (i, d) in list.iter().enumerate() { println!("[{}] {}", i, d); }
        }
        #[cfg(feature = "vulkan")]
        Command::VkTestShaders => {
            use stylize_core::render::vk;
            let ctx = vk::VkContext::new("stylize-test-shaders")?;
            let mod_vert = vk::create_shader_module(&ctx.device, vk::OUTLINE_VERT_SPV)?;
            let mod_frag = vk::create_shader_module(&ctx.device, vk::TOON_FRAG_SPV)?;
            println!("Created shader modules: vert={:?}, frag={:?}", mod_vert, mod_frag);
            unsafe {
                ctx.device.destroy_shader_module(mod_vert, None);
                ctx.device.destroy_shader_module(mod_frag, None);
            }
            println!("Shader modules destroyed successfully.");
        }
        #[cfg(feature = "vulkan")]
        Command::VkRenderTest { width, height, out } => {
            use stylize_core::render::vk;
            let ctx = vk::VkContext::new("stylize-render-test")?;
            let pixels = vk::render_offscreen_rgba(&ctx, width, height)?;
            let img = image::RgbaImage::from_raw(width, height, pixels)
                .ok_or_else(|| anyhow::anyhow!("Failed to create image from raw"))?;
            img.save(&out)?;
            println!("Wrote {}x{} image to {}", width, height, out);
        }
        #[cfg(feature = "vulkan")]
        Command::VkGbufferTest { width, height, out_prefix } => {
            use stylize_core::render::vk;
            let ctx = vk::VkContext::new("stylize-gbuffer-test")?;
            let (albedo, normal) = vk::render_gbuffer_offscreen(&ctx, width, height)?;
            let img_a = image::RgbaImage::from_raw(width, height, albedo)
                .ok_or_else(|| anyhow::anyhow!("Failed to create albedo image"))?;
            let img_n = image::RgbaImage::from_raw(width, height, normal)
                .ok_or_else(|| anyhow::anyhow!("Failed to create normal image"))?;
            let ap = format!("{}-albedo.png", out_prefix);
            let np = format!("{}-normal.png", out_prefix);
            img_a.save(&ap)?;
            img_n.save(&np)?;
            println!("Wrote {} and {}", ap, np);
        }
        #[cfg(feature = "vulkan")]
        Command::VkToonFromGbuf { width, height, out } => {
            use stylize_core::render::vk;
            let ctx = vk::VkContext::new("stylize-toon-from-gbuf")?;
            let pixels = vk::render_toon_from_gbuffer(&ctx, width, height)?;
            let img = image::RgbaImage::from_raw(width, height, pixels)
                .ok_or_else(|| anyhow::anyhow!("Failed to create image from raw"))?;
            img.save(&out)?;
            println!("Wrote {}x{} image to {}", width, height, out);
        }
        #[cfg(feature = "vulkan")]
        Command::VkGbufferMesh { width, height, out_prefix } => {
            use stylize_core::render::vk;
            let ctx = vk::VkContext::new("stylize-gbuffer-mesh")?;
            let (albedo, normal) = vk::render_mesh_gbuffer_offscreen(&ctx, width, height)?;
            let img_a = image::RgbaImage::from_raw(width, height, albedo)
                .ok_or_else(|| anyhow::anyhow!("Failed to create albedo image"))?;
            let img_n = image::RgbaImage::from_raw(width, height, normal)
                .ok_or_else(|| anyhow::anyhow!("Failed to create normal image"))?;
            let ap = format!("{}-albedo.png", out_prefix);
            let np = format!("{}-normal.png", out_prefix);
            img_a.save(&ap)?;
            img_n.save(&np)?;
            println!("Wrote {} and {}", ap, np);
        }
        #[cfg(feature = "vulkan")]
        Command::VkToonMesh { width, height, out } => {
            use stylize_core::render::vk;
            let ctx = vk::VkContext::new("stylize-toon-mesh")?;
            let pixels = vk::render_toon_from_mesh(&ctx, width, height)?;
            let img = image::RgbaImage::from_raw(width, height, pixels)
                .ok_or_else(|| anyhow::anyhow!("Failed to create image from raw"))?;
            img.save(&out)?;
            println!("Wrote {}x{} image to {}", width, height, out);
        }
    }
    Ok(())
}
