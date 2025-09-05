#version 450

layout(location=0) in vec3 vNormal;
layout(location=1) in vec3 vView;
layout(location=2) in vec3 vLight;
layout(location=3) in vec3 vBaseColor;

layout(location=0) out vec4 oColor;

layout(push_constant) uniform ToonParams {
    float shadowThreshold;   // e.g. 0.6 face, 0.55 cloth
    float midThreshold;      // optional third band (<=0 to disable)
    float rimStrength;       // 0..1
    float rimWidth;          // 0..1
} P;

vec3 toonRamp(vec3 N, vec3 L, vec3 V, vec3 baseColor) {
    float ndl = max(dot(normalize(N), normalize(L)), 0.0);

    // Base 2-band
    float band = ndl < P.shadowThreshold ? 0.0 : 1.0;

    // Optional 3rd band
    if (P.midThreshold > 0.0) {
        band = ndl < P.shadowThreshold ? 0.0
              : (ndl < P.midThreshold ? 0.5 : 1.0);
    }

    // Rim light
    float rim = pow(clamp(1.0 - max(dot(normalize(N), normalize(V)), 0.0), 0.0, 1.0), 1.0 / max(P.rimWidth, 0.001));
    float rimMask = smoothstep(0.8, 1.0, rim) * P.rimStrength;

    vec3 lit = baseColor * (0.4 + 0.6 * band);
    return mix(lit, baseColor, rimMask);
}

void main() {
    vec3 c = toonRamp(vNormal, vLight, vView, vBaseColor);
    oColor = vec4(c, 1.0);
}

