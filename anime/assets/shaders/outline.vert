#version 450

layout(location=0) in vec3 inPos;
layout(location=1) in vec3 inNormal;

layout(push_constant) uniform LinePC { float outlineWidth; } LPC;

void main() {
    vec3 offset = inNormal * LPC.outlineWidth;
    gl_Position = vec4(inPos + offset, 1.0);
}
