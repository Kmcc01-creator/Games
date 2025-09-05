#version 450

layout(location=0) in vec3 vNormal;
layout(location=0) out vec4 oAlbedo;
layout(location=1) out vec4 oNormal;

void main() {
    vec3 n = normalize(vNormal);
    // Simple base color; you can swap to normal-based coloring for debug
    vec3 base = vec3(0.65, 0.30, 0.85);
    oAlbedo = vec4(base, 1.0);
    oNormal = vec4(n * 0.5 + 0.5, 1.0);
}

