#version 450
layout(location=0) in vec2 vUV;
layout(location=0) out vec4 outColor;

layout(set=0, binding=0) uniform UBO { vec4 tint; } ubo;
layout(set=0, binding=1) uniform sampler2D tex;

void main(){
  vec4 texel = texture(tex, vUV);
  outColor = texel * ubo.tint;
}
