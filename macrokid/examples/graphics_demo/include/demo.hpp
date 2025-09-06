#pragma once

struct __attribute__((annotate("mk::struct(tag=vertex)"))) DemoVertex {
    float pos[3];
    float __attribute__((annotate("mk::vertex(location=1,format=vec3)"))) normal[3];
    float __attribute__((annotate("mk::vertex(location=2,format=vec2)"))) uv[2];
};

struct Material {
    int __attribute__((annotate("mk::resource(kind=uniform,set=0,binding=0)"))) id;
};
