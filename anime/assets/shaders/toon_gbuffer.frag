#version 450

layout(location=0) in vec2 vUv;
layout(location=0) out vec4 oColor;

layout(set=0, binding=0) uniform sampler2D uAlbedo;
layout(set=0, binding=1) uniform sampler2D uNormal;
layout(set=0, binding=2) uniform usampler2D uMaterial; // R8_UINT
layout(std140, set=0, binding=3) uniform MaterialLUT {
    vec4 row0[8]; // shadow, mid, rimStrength, rimWidth
    vec4 row1[8]; // softness, hueShad, hueLit, satShad
    vec4 row2[8]; // satLit, specThr, specInt, pad
} MLUT;

// Extended style push-constants (48 bytes: 12 floats)
layout(push_constant) uniform ToonParams {
    float shadowThreshold;
    float midThreshold;
    float rimStrength;
    float rimWidth;
    float bandSoftness;
    float hueShiftShadowDeg;
    float hueShiftLightDeg;
    float satScaleShadow;
    float satScaleLight;
    float specThreshold;
    float specIntensity;
    float _pad;
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

vec3 toonStylizeParams(
    vec3 N, vec3 L, vec3 V, vec3 baseColor,
    float shadowThreshold, float midThreshold,
    float rimStrength, float rimWidth, float bandSoftness,
    float hueShiftShadowDeg, float hueShiftLightDeg,
    float satScaleShadow, float satScaleLight,
    float specThreshold, float specIntensity
) {
    N = normalize(N); L = normalize(L); V = normalize(V);
    float ndl = max(dot(N, L), 0.0);
    float ndv = max(dot(N, V), 0.0);
    vec3 H = normalize(L + V);
    float ndh = max(dot(N, H), 0.0);

    float s = max(bandSoftness, 1e-3);
    float band = smoothstep(shadowThreshold - s, shadowThreshold + s, ndl);
    if (midThreshold > 0.0) {
        float mid = smoothstep(midThreshold - s, midThreshold + s, ndl);
        band = mix(0.0, 0.5, band);
        band = mix(band, 1.0, mid);
    }

    float hueShift = mix(hueShiftShadowDeg, hueShiftLightDeg, band);
    float satScale = mix(satScaleShadow, satScaleLight, band);
    vec3 ramped = applyHueSat(baseColor, hueShift, satScale);

    vec3 lit = ramped * (0.35 + 0.65 * band);

    float rim = pow(clamp(1.0 - ndv, 0.0, 1.0), 1.0 / max(rimWidth, 0.001));
    float rimMask = smoothstep(0.8, 1.0, rim) * rimStrength;
    vec3 withRim = mix(lit, baseColor, rimMask);

    float specMask = smoothstep(specThreshold, min(specThreshold + 0.02, 1.0), ndh);
    vec3 spec = vec3(specIntensity) * specMask;
    return clamp(withRim + spec, 0.0, 1.0);
}

void main() {
    vec3 base = texture(uAlbedo, vUv).rgb;
    vec3 n = texture(uNormal, vUv).rgb * 2.0 - 1.0; // unpack
    vec3 V = vec3(0.0, 0.0, 1.0);
    vec3 L = normalize(vec3(0.6, 0.6, 0.2));
    uint id = texture(uMaterial, vUv).r;
    id = min(id, uint(7));
    vec4 r0 = MLUT.row0[int(id)];
    vec4 r1 = MLUT.row1[int(id)];
    vec4 r2 = MLUT.row2[int(id)];
    vec3 c = toonStylizeParams(
        n, L, V, base,
        r0.x, r0.y, r0.z, r0.w,
        r1.x, r1.y, r1.z, r1.w,
        r2.x, r2.y, r2.z
    );
    oColor = vec4(c, 1.0);
}
