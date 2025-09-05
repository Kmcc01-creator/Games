#version 450

layout(location=0) in vec3 vNormal;
layout(location=1) in vec3 vView;
layout(location=2) in vec3 vLight;
layout(location=3) in vec3 vBaseColor;

layout(location=0) out vec4 oColor;

// Extended style push-constants (48 bytes: 12 floats)
layout(push_constant) uniform ToonParams {
    float shadowThreshold;   // e.g., 0.6
    float midThreshold;      // <= 0 to disable
    float rimStrength;       // 0..1
    float rimWidth;          // 0..1 (controls falloff of rim mask)
    float bandSoftness;      // small > 0 for stable edges (e.g., 0.05)
    float hueShiftShadowDeg; // negative = cooler
    float hueShiftLightDeg;  // positive = warmer
    float satScaleShadow;    // e.g., 0.95
    float satScaleLight;     // e.g., 1.05
    float specThreshold;     // e.g., 0.86
    float specIntensity;     // e.g., 0.25
    float _pad;              // unused, keeps 16B alignment
} P;

vec3 rgb2hsv(vec3 c) {
    vec4 K = vec4(0.0, -1.0/3.0, 2.0/3.0, -1.0);
    vec4 p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
    vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));
    float d = q.x - min(q.w, q.y);
    float e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

vec3 hsv2rgb(vec3 c) {
    vec3 rgb = clamp(abs(mod(c.x * 6.0 + vec3(0.0, 4.0, 2.0), 6.0) - 3.0) - 1.0, 0.0, 1.0);
    return c.z * mix(vec3(1.0), rgb, c.y);
}

vec3 applyHueSat(vec3 rgb, float hueShiftDeg, float satScale) {
    vec3 hsv = rgb2hsv(rgb);
    hsv.x = fract(hsv.x + hueShiftDeg / 360.0);
    hsv.y = clamp(hsv.y * satScale, 0.0, 1.0);
    return hsv2rgb(hsv);
}

vec3 toonStylize(vec3 N, vec3 L, vec3 V, vec3 baseColor) {
    N = normalize(N); L = normalize(L); V = normalize(V);
    float ndl = max(dot(N, L), 0.0);
    float ndv = max(dot(N, V), 0.0);
    vec3 H = normalize(L + V);
    float ndh = max(dot(N, H), 0.0);

    float s = max(P.bandSoftness, 1e-3);
    float band = smoothstep(P.shadowThreshold - s, P.shadowThreshold + s, ndl);
    if (P.midThreshold > 0.0) {
        float mid = smoothstep(P.midThreshold - s, P.midThreshold + s, ndl);
        band = mix(0.0, 0.5, band);
        band = mix(band, 1.0, mid);
    }

    float hueShift = mix(P.hueShiftShadowDeg, P.hueShiftLightDeg, band);
    float satScale = mix(P.satScaleShadow, P.satScaleLight, band);
    vec3 ramped = applyHueSat(baseColor, hueShift, satScale);

    vec3 lit = ramped * (0.35 + 0.65 * band);

    float rim = pow(clamp(1.0 - ndv, 0.0, 1.0), 1.0 / max(P.rimWidth, 0.001));
    float rimMask = smoothstep(0.8, 1.0, rim) * P.rimStrength;
    vec3 withRim = mix(lit, baseColor, rimMask);

    float specMask = smoothstep(P.specThreshold, min(P.specThreshold + 0.02, 1.0), ndh);
    vec3 spec = vec3(P.specIntensity) * specMask;
    return clamp(withRim + spec, 0.0, 1.0);
}

void main() {
    vec3 c = toonStylize(vNormal, vLight, vView, vBaseColor);
    oColor = vec4(c, 1.0);
}
