#version 450

// Vertex attributes
layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec4 inTangent; // w component is handedness
layout(location = 3) in vec2 inUV;

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
layout(location = 2) out vec3 fragTangent;
layout(location = 3) out vec3 fragBitangent;
layout(location = 4) out vec2 fragUV;
layout(location = 5) out vec4 fragLightSpacePos; // For shadow mapping

// Shadow map light matrix (could be in UBO if more complex)
layout(set = 0, binding = 2) uniform ShadowUBO {
    mat4 lightSpaceMatrix;
} shadow;

void main() {
    // Transform position to world space
    vec4 worldPos = push.model * vec4(inPosition, 1.0);
    fragWorldPos = worldPos.xyz;
    
    // Transform normal and tangent to world space
    fragNormal = normalize((push.normalMatrix * vec4(inNormal, 0.0)).xyz);
    fragTangent = normalize((push.normalMatrix * vec4(inTangent.xyz, 0.0)).xyz);
    
    // Calculate bitangent using cross product and handedness
    fragBitangent = cross(fragNormal, fragTangent) * inTangent.w;
    
    // Pass through UV coordinates
    fragUV = inUV;
    
    // Calculate position in light space for shadow mapping
    fragLightSpacePos = shadow.lightSpaceMatrix * worldPos;
    
    // Transform to clip space
    gl_Position = camera.proj * camera.view * worldPos;
}