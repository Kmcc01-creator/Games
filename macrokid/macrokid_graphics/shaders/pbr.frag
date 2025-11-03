#version 450

// Inputs from vertex shader
layout(location = 0) in vec3 fragWorldPos;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in vec3 fragTangent;
layout(location = 3) in vec3 fragBitangent;
layout(location = 4) in vec2 fragUV;
layout(location = 5) in vec4 fragLightSpacePos;

// Output
layout(location = 0) out vec4 outColor;

// Light data
struct DirectionalLight {
    vec3 direction;
    float intensity;
    vec3 color;
    float padding;
};

struct PointLight {
    vec3 position;
    float range;
    vec3 color;
    float intensity;
};

layout(set = 0, binding = 1) uniform LightsUBO {
    DirectionalLight dirLight;
    PointLight pointLights[3];
    vec3 viewPos; // Camera position
    float padding;
} lights;

// Material properties
layout(set = 1, binding = 0) uniform MaterialUBO {
    vec3 baseColorFactor;
    float metallicFactor;
    float roughnessFactor;
    float normalScale;
    float occlusionStrength;
    float emissiveFactor;
} material;

// Textures
layout(set = 1, binding = 1) uniform sampler2D albedoMap;
layout(set = 1, binding = 2) uniform sampler2D normalMap;
layout(set = 1, binding = 3) uniform sampler2D metallicRoughnessMap;
layout(set = 1, binding = 4) uniform sampler2D aoMap;

// Environment mapping
layout(set = 2, binding = 0) uniform samplerCube environmentCube;
layout(set = 2, binding = 1) uniform sampler2D shadowMap;

// Constants
const float PI = 3.14159265359;
const float MIN_ROUGHNESS = 0.04;

// PBR utility functions
vec3 getNormalFromMap() {
    vec3 tangentNormal = texture(normalMap, fragUV).xyz * 2.0 - 1.0;
    tangentNormal.xy *= material.normalScale;
    
    vec3 N = normalize(fragNormal);
    vec3 T = normalize(fragTangent);
    vec3 B = normalize(fragBitangent);
    mat3 TBN = mat3(T, B, N);
    
    return normalize(TBN * tangentNormal);
}

float distributionGGX(vec3 N, vec3 H, float roughness) {
    float a = roughness * roughness;
    float a2 = a * a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;
    
    float nom = a2;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;
    
    return nom / denom;
}

float geometrySchlickGGX(float NdotV, float roughness) {
    float r = (roughness + 1.0);
    float k = (r * r) / 8.0;
    
    float nom = NdotV;
    float denom = NdotV * (1.0 - k) + k;
    
    return nom / denom;
}

float geometrySmith(vec3 N, vec3 V, vec3 L, float roughness) {
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    float ggx2 = geometrySchlickGGX(NdotV, roughness);
    float ggx1 = geometrySchlickGGX(NdotL, roughness);
    
    return ggx1 * ggx2;
}

vec3 fresnelSchlick(float cosTheta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

vec3 fresnelSchlickRoughness(float cosTheta, vec3 F0, float roughness) {
    return F0 + (max(vec3(1.0 - roughness), F0) - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

float calculateShadow(vec4 fragPosLightSpace, vec3 normal, vec3 lightDir) {
    // Perspective divide
    vec3 projCoords = fragPosLightSpace.xyz / fragPosLightSpace.w;
    
    // Transform to [0,1] range
    projCoords = projCoords * 0.5 + 0.5;
    
    // Get closest depth value from light's perspective
    float closestDepth = texture(shadowMap, projCoords.xy).r;
    
    // Get depth of current fragment from light's perspective
    float currentDepth = projCoords.z;
    
    // Calculate bias to prevent shadow acne
    float bias = max(0.05 * (1.0 - dot(normal, lightDir)), 0.005);
    
    // PCF (Percentage-Closer Filtering) for softer shadows
    float shadow = 0.0;
    vec2 texelSize = 1.0 / textureSize(shadowMap, 0);
    for(int x = -1; x <= 1; ++x) {
        for(int y = -1; y <= 1; ++y) {
            float pcfDepth = texture(shadowMap, projCoords.xy + vec2(x, y) * texelSize).r;
            shadow += currentDepth - bias > pcfDepth ? 1.0 : 0.0;
        }
    }
    shadow /= 9.0;
    
    // Keep shadows in range
    if(projCoords.z > 1.0) shadow = 0.0;
    
    return shadow;
}

vec3 calculateDirectionalLight(DirectionalLight light, vec3 normal, vec3 viewDir, vec3 albedo, float metallic, float roughness, vec3 F0) {
    vec3 lightDir = normalize(-light.direction);
    vec3 halfwayDir = normalize(lightDir + viewDir);
    
    // Calculate radiance
    vec3 radiance = light.color * light.intensity;
    
    // BRDF components
    float NDF = distributionGGX(normal, halfwayDir, roughness);
    float G = geometrySmith(normal, viewDir, lightDir, roughness);
    vec3 F = fresnelSchlick(max(dot(halfwayDir, viewDir), 0.0), F0);
    
    vec3 numerator = NDF * G * F;
    float denominator = 4.0 * max(dot(normal, viewDir), 0.0) * max(dot(normal, lightDir), 0.0) + 0.0001;
    vec3 specular = numerator / denominator;
    
    vec3 kS = F;
    vec3 kD = vec3(1.0) - kS;
    kD *= 1.0 - metallic;
    
    float NdotL = max(dot(normal, lightDir), 0.0);
    
    // Calculate shadow
    float shadow = calculateShadow(fragLightSpacePos, normal, lightDir);
    
    return (kD * albedo / PI + specular) * radiance * NdotL * (1.0 - shadow);
}

vec3 calculatePointLight(PointLight light, vec3 normal, vec3 viewDir, vec3 fragPos, vec3 albedo, float metallic, float roughness, vec3 F0) {
    vec3 lightDir = normalize(light.position - fragPos);
    vec3 halfwayDir = normalize(lightDir + viewDir);
    
    // Attenuation
    float distance = length(light.position - fragPos);
    float attenuation = 1.0 / (1.0 + 0.09 * distance + 0.032 * distance * distance);
    attenuation *= max(0.0, 1.0 - (distance / light.range));
    
    // Calculate radiance
    vec3 radiance = light.color * light.intensity * attenuation;
    
    // BRDF components
    float NDF = distributionGGX(normal, halfwayDir, roughness);
    float G = geometrySmith(normal, viewDir, lightDir, roughness);
    vec3 F = fresnelSchlick(max(dot(halfwayDir, viewDir), 0.0), F0);
    
    vec3 numerator = NDF * G * F;
    float denominator = 4.0 * max(dot(normal, viewDir), 0.0) * max(dot(normal, lightDir), 0.0) + 0.0001;
    vec3 specular = numerator / denominator;
    
    vec3 kS = F;
    vec3 kD = vec3(1.0) - kS;
    kD *= 1.0 - metallic;
    
    float NdotL = max(dot(normal, lightDir), 0.0);
    
    return (kD * albedo / PI + specular) * radiance * NdotL;
}

void main() {
    // Sample textures
    vec3 albedo = pow(texture(albedoMap, fragUV).rgb * material.baseColorFactor, vec3(2.2));
    vec2 metallicRoughness = texture(metallicRoughnessMap, fragUV).rg;
    float metallic = metallicRoughness.r * material.metallicFactor;
    float roughness = max(metallicRoughness.g * material.roughnessFactor, MIN_ROUGHNESS);
    float ao = texture(aoMap, fragUV).r;
    ao = mix(1.0, ao, material.occlusionStrength);
    
    // Calculate normal
    vec3 normal = getNormalFromMap();
    vec3 viewDir = normalize(lights.viewPos - fragWorldPos);
    
    // Calculate reflectance at normal incidence
    vec3 F0 = vec3(0.04);
    F0 = mix(F0, albedo, metallic);
    
    // Lighting calculation
    vec3 Lo = vec3(0.0);
    
    // Directional light
    Lo += calculateDirectionalLight(lights.dirLight, normal, viewDir, albedo, metallic, roughness, F0);
    
    // Point lights
    for(int i = 0; i < 3; ++i) {
        Lo += calculatePointLight(lights.pointLights[i], normal, viewDir, fragWorldPos, albedo, metallic, roughness, F0);
    }
    
    // Ambient lighting (environment mapping)
    vec3 F = fresnelSchlickRoughness(max(dot(normal, viewDir), 0.0), F0, roughness);
    vec3 kS = F;
    vec3 kD = 1.0 - kS;
    kD *= 1.0 - metallic;
    
    // Sample environment map
    vec3 irradiance = texture(environmentCube, normal).rgb;
    vec3 diffuse = irradiance * albedo;
    
    // Reflection
    vec3 R = reflect(-viewDir, normal);
    vec3 prefilteredColor = texture(environmentCube, R).rgb;
    vec3 specular = prefilteredColor * F;
    
    vec3 ambient = (kD * diffuse + specular) * ao * 0.3; // Scale ambient contribution
    
    vec3 color = ambient + Lo;
    
    // HDR tone mapping
    color = color / (color + vec3(1.0));
    
    // Gamma correction
    color = pow(color, vec3(1.0 / 2.2));
    
    outColor = vec4(color, 1.0);
}