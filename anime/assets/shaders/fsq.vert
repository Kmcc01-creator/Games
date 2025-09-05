#version 450

// Full-screen triangle with UVs for sampling
layout(location=0) out vec2 vUv;

void main() {
    const vec2 pos[3] = vec2[3](
        vec2(-1.0, -1.0),
        vec2( 3.0, -1.0),
        vec2(-1.0,  3.0)
    );
    vec2 p = pos[gl_VertexIndex];
    gl_Position = vec4(p, 0.0, 1.0);
    vUv = p * 0.5 + 0.5;
}

