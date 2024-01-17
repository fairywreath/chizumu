#version 460 core

#pragma shader_stage(vertex)

layout(location = 0) in vec3 position;
layout(location = 1) in vec4 color;

layout(location = 0) out vec4 outColor;

layout(std140, binding = 0) uniform SceneConstants
{
    mat4 viewProj;
};

void main()
{
    gl_Position = viewProj * vec4(position, 1.0);
    // gl_Position = vec4(position, 1.0);
    outColor = color;
}