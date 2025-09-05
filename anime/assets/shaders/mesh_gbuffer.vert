#version 450

layout(location=0) in vec3 inPos;
layout(location=1) in vec3 inNormal;

layout(location=0) out vec3 vNormal;

void main() {
    gl_Position = vec4(inPos, 1.0);
    vNormal = normalize(inNormal);
}

