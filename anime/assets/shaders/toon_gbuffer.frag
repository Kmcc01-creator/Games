#version 450

layout(location=0) in vec2 vUv;
layout(location=0) out vec4 oColor;

layout(set=0, binding=0) uniform sampler2D uAlbedo;
layout(set=0, binding=1) uniform sampler2D uNormal;

layout(push_constant) uniform ToonParams {
    float shadowThreshold;   // e.g. 0.6 face, 0.55 cloth
    float midThreshold;      // optional third band (<=0 to disable)
    float rimStrength;       // 0..1
    float rimWidth;          // 0..1
} P;

vec3 toonRamp(vec3 N, vec3 L, vec3 V, vec3 baseColor) {
    float ndl = max(dot(normalize(N), normalize(L)), 0.0);

    float band = ndl < P.shadowThreshold ? 0.0 : 1.0;
    if (P.midThreshold > 0.0) {
        band = ndl < P.shadowThreshold ? 0.0
              : (ndl < P.midThreshold ? 0.5 : 1.0);
    }

    float rim = pow(clamp(1.0 - max(dot(normalize(N), normalize(V)), 0.0), 0.0, 1.0), 1.0 / max(P.rimWidth, 0.001));
    float rimMask = smoothstep(0.8, 1.0, rim) * P.rimStrength;

    vec3 lit = baseColor * (0.4 + 0.6 * band);
    return mix(lit, baseColor, rimMask);
}

void main() {
    vec3 base = texture(uAlbedo, vUv).rgb;
    vec3 n = texture(uNormal, vUv).rgb * 2.0 - 1.0; // unpack 0..1 -> -1..1
    vec3 V = vec3(0.0, 0.0, 1.0);
    // Use a shallower light (low Z) so slopes affect ndl strongly
    vec3 L = normalize(vec3(0.6, 0.6, 0.2));
    vec3 c = toonRamp(n, L, V, base);
    oColor = vec4(c, 1.0);
}
