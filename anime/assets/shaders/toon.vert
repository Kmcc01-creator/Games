#version 450

// Full-screen triangle generator + simple varyings for toon.frag
layout(location=0) out vec3 vNormal;
layout(location=1) out vec3 vView;
layout(location=2) out vec3 vLight;
layout(location=3) out vec3 vBaseColor;

void main() {
    // Full-screen triangle positions based on gl_VertexIndex
    const vec2 pos[3] = vec2[3](
        vec2(-1.0, -1.0),
        vec2( 3.0, -1.0),
        vec2(-1.0,  3.0)
    );
    vec2 p = pos[gl_VertexIndex];
    gl_Position = vec4(p, 0.0, 1.0);

    // Vary normal across screen so toon bands are visible
    vNormal = normalize(vec3(p, 1.0));
    vView = normalize(vec3(0.0, 0.0, 1.0));
    vLight = normalize(vec3(0.5, 0.5, 1.0));
    vBaseColor = vec3(0.2, 0.4, 0.9);
}
