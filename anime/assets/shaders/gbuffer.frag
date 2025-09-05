#version 450

layout(location=0) in vec2 vUv;

// Multiple render targets: 0 = albedo/region, 1 = normal (packed 0..1)
layout(location=0) out vec4 oAlbedo;
layout(location=1) out vec4 oNormal;
layout(location=2) out uint oMaterial; // R8_UINT

// Synthetic G-buffer content for demo:
// - Albedo: soft checkerboard + gradient
// - Normal: derive from a simple heightfield to show variation

float height(vec2 uv) {
    return 0.15 * sin(6.2831 * uv.x) * cos(6.2831 * uv.y);
}

void main() {
    // Albedo: checker pattern
    vec2 chk = floor(vUv * 8.0);
    float checker = mod(chk.x + chk.y, 2.0);
    vec3 base = mix(vec3(0.85, 0.82, 0.78), vec3(0.20, 0.24, 0.30), checker);
    base *= mix(0.9, 1.1, vUv.y);
    oAlbedo = vec4(base, 1.0);

    // Normal: synthetic from UV for strong, obvious variation
    // Centered uv in [-1,1]
    vec2 cuv = vUv * 2.0 - 1.0;
    // Tilt surface so normals vary across screen
    vec3 n = normalize(vec3(cuv.x, cuv.y, 0.35));
    // Pack into 0..1
    oNormal = vec4(n * 0.5 + 0.5, 1.0);

    // Material ID: alternate by checkerboard (0=skin, 2=cloth)
    oMaterial = checker < 0.5 ? uint(0) : uint(2);
}
