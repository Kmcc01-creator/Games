pub struct ShaderSources {
    pub vs: &'static str,
    pub fs: &'static str,
}

pub trait LightingModel {
    fn shader_sources() -> ShaderSources;
}

// Associates a ResourceBindings type with the lighting model
pub trait HasBindings {
    type RB: macrokid_graphics::resources::ResourceBindings;
}

pub trait LightSetup {}

// Minimal built-in shader snippets for a forward Phong MVP
pub mod default_shaders {
    // Set/binding conventions expected by generated ResourceBindings:
    // set=0, binding=0: uniform buffer with MVP + light params
    // set=0, binding=1: combined image sampler for albedo
    pub const VS_POS_UV: &str = r#"#version 450
layout(location=0) in vec3 a_pos;
layout(location=1) in vec3 a_normal;
layout(location=2) in vec2 a_uv;
layout(location=0) out vec3 v_normal;
layout(location=1) out vec2 v_uv;
    layout(set = 0, binding = 0) uniform Scene {
        mat4 mvp;
        vec3 light_dir; float _pad0;
        vec3 light_color; float _pad1;
    } uScene;
void main() {
    v_normal = a_normal;
    v_uv = a_uv;
    gl_Position = uScene.mvp * vec4(a_pos, 1.0);
}

// Convenience: build a PipelineDesc for a forward lighting pass from a LightingModel
pub fn forward_pipeline_desc_for<M: LightingModel>(name: &str) -> macrokid_graphics::pipeline::PipelineDesc {
    use macrokid_graphics::pipeline::*;
    let ss = M::shader_sources();
    let vs_prefixed = format!("inline.vert:{}", ss.vs);
    let fs_prefixed = format!("inline.frag:{}", ss.fs);
    let vs_static: &'static str = Box::leak(vs_prefixed.into_boxed_str());
    let fs_static: &'static str = Box::leak(fs_prefixed.into_boxed_str());
    PipelineDesc {
        name: Box::leak(name.to_string().into_boxed_str()),
        shaders: ShaderPaths { vs: vs_static, fs: fs_static },
        topology: Topology::TriangleList,
        depth: true,
        raster: Some(RasterState { polygon: PolygonMode::Fill, cull: CullMode::Back, front_face: FrontFace::Cw }),
        blend: Some(ColorBlendState { enable: false }),
        samples: Some(1),
        depth_stencil: Some(DepthState { test: true, write: true, compare: CompareOp::LessOrEqual }),
        dynamic: Some(DynamicStateDesc { viewport: true, scissor: true }),
        push_constants: None,
        color_targets: None,
        depth_target: Some(DepthTargetDesc { format: "D32_SFLOAT" }),
    }
}

// Convenience: return both a synthesized PipelineDesc and a phantom for the RB type
pub fn forward_pipeline_and_rb<M>(name: &str) -> (macrokid_graphics::pipeline::PipelineDesc, core::marker::PhantomData<<M as HasBindings>::RB>)
where
    M: LightingModel + HasBindings,
{
    (forward_pipeline_desc_for::<M>(name), core::marker::PhantomData::<M::RB>)
}
"#;

    pub const FS_PHONG_MIN: &str = r#"#version 450
layout(location=0) in vec3 v_normal;
layout(location=1) in vec2 v_uv;
layout(location=0) out vec4 o_color;
    layout(set = 0, binding = 0) uniform Scene {
        mat4 mvp;
        vec3 light_dir; float _pad0;
        vec3 light_color; float _pad1;
    } uScene;
    layout(set = 0, binding = 1) uniform sampler2D uAlbedo;
    void main() {
        vec3 N = normalize(v_normal);
        vec3 L = normalize(uScene.light_dir);
        vec3 V = normalize(vec3(0.0, 0.0, 1.0));
        float NdotL = max(dot(N,L), 0.0);
        vec3 albedo = texture(uAlbedo, v_uv).rgb;
        vec3 diffuse = albedo * uScene.light_color * NdotL;
        vec3 R = reflect(-L, N);
        float spec = pow(max(dot(R, V), 0.0), 32.0);
        vec3 specular = vec3(0.5) * spec;
        vec3 ambient = 0.05 * albedo;
        o_color = vec4(ambient + diffuse + specular, 1.0);
    }
"#;
}
