#version 450

// Inputs from vertex shader
layout(location = 0) in vec3 fragWorldPos;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in vec2 fragUV;

// Output
layout(location = 0) out vec4 outColor;

// Light data
layout(set = 0, binding = 1) uniform LightUBO {
    vec3 direction;
    float intensity;
    vec3 color;
    float padding;
} light;

// Textures
layout(set = 1, binding = 0) uniform sampler2D diffuseTexture;

void main() {
    // Sample the texture
    vec4 texColor = texture(diffuseTexture, fragUV);
    
    // Simple Lambert lighting
    vec3 normal = normalize(fragNormal);
    vec3 lightDir = normalize(-light.direction);
    
    float NdotL = max(dot(normal, lightDir), 0.0);
    
    // Simple ambient + diffuse lighting
    vec3 ambient = texColor.rgb * 0.2;
    vec3 diffuse = texColor.rgb * light.color * light.intensity * NdotL;
    
    vec3 finalColor = ambient + diffuse;
    
    // Simple gamma correction
    finalColor = pow(finalColor, vec3(1.0 / 2.2));
    
    outColor = vec4(finalColor, texColor.a);
}