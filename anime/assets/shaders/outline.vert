#version 450

layout(location=0) in vec3 inPos;
layout(location=1) in vec3 inNormal;

layout(push_constant) uniform LinePC { float outlineWidth; } LPC;

layout(set=0, binding=0) uniform MVPBlock {
    mat4 MVP;
} U;

void main() {
    vec3 offset = inNormal * LPC.outlineWidth;
    gl_Position = U.MVP * vec4(inPos + offset, 1.0);
}

