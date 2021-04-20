#version 460

#pragma shader_stage(vertex)

#extension GL_ARB_separate_shader_objects : enable
#extension GL_GOOGLE_include_directive : require

#include "common.glsl"

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;

layout(location = 0) out vec3 fragColor;

layout(set = 0, binding = 0, std430) buffer DrawDataList {
  DrawData drawData[];
};

layout(set = 0, binding = 1, std140) uniform Constants {
  mat4 view;
  mat4 previous_view;
  mat4 projection;
  mat4 viewProjection;
  float znear, zfar, w, h;
  uint objectCount;
};

void main() {
  DrawData data = drawData[gl_BaseInstance];
  gl_Position = viewProjection * data.transform * vec4(inPosition, 1.0);
  fragColor = inColor;
}
