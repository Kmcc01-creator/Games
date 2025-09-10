#version 450
layout(location=0) out vec2 vUV;
void main(){
  // Fullscreen-ish triangle via gl_VertexIndex
  const vec2 pos[3] = vec2[3](vec2(-1.0,-1.0), vec2(3.0,-1.0), vec2(-1.0,3.0));
  gl_Position = vec4(pos[gl_VertexIndex], 0.0, 1.0);
  // Map to [0,1] UVs
  vUV = (pos[gl_VertexIndex] * 0.5) + 0.5;
}
