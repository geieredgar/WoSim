#version 460

#pragma shader_stage(fragment)

layout(location = 0) in vec2 inUV;
layout(location = 1) in vec4 inColor;

layout(location = 0) out vec4 outColor;

layout(set = 0, binding = 0) uniform sampler2D fontTexture;

layout(push_constant) uniform Constants {
    uint textureId;
};

void main() {
    outColor = inColor * texture(fontTexture, inUV);
}
