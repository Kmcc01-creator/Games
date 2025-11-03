#version 450

// Vertex attributes (only position needed for shadow mapping)
layout(location = 0) in vec3 inPosition;

// Uniform buffer for light space matrix
layout(set = 0, binding = 0) uniform LightMatrixUBO {
    mat4 lightSpaceMatrix;
} lightMatrix;

// Push constants for per-object transforms
layout(push_constant) uniform PushConstants {
    mat4 model;
} push;

void main() {
    // Transform vertex to light space
    vec4 worldPos = push.model * vec4(inPosition, 1.0);
    gl_Position = lightMatrix.lightSpaceMatrix * worldPos;
}