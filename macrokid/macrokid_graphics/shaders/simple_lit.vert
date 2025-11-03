#version 450

// Vertex attributes
layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;

// Uniform buffers
layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view;
    mat4 proj;
} camera;

// Push constants for per-object transforms
layout(push_constant) uniform PushConstants {
    mat4 model;
    mat4 normalMatrix;
} push;

// Outputs to fragment shader
layout(location = 0) out vec3 fragWorldPos;
layout(location = 1) out vec3 fragNormal;
layout(location = 2) out vec2 fragUV;

void main() {
    // Transform position to world space
    vec4 worldPos = push.model * vec4(inPosition, 1.0);
    fragWorldPos = worldPos.xyz;
    
    // Transform normal to world space
    fragNormal = normalize((push.normalMatrix * vec4(inNormal, 0.0)).xyz);
    
    // Pass through UV coordinates
    fragUV = inUV;
    
    // Transform to clip space
    gl_Position = camera.proj * camera.view * worldPos;
}