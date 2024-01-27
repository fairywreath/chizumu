#version 460 core

#pragma shader_stage(vertex)

layout(location = 0) in vec3 position;

layout(location = 0) out vec4 color;

struct HitInstanceData
{
    mat4 model;
    vec4 color;
};

layout(std140, binding = 0) uniform GlobalSceneUbo
{
    mat4 viewProj;
}
global;

layout(std140, binding = 1) uniform RunnerSceneUbo
{
    mat4 transform;
}
runner;

layout(std430, binding = 2) readonly buffer HitInstanceDataSbo
{
    HitInstanceData instances[];
};

void main()
{
    HitInstanceData instanceData = instances[gl_InstanceIndex];

    gl_Position = global.viewProj * runner.transform * instanceData.model * vec4(position, 1.0);
    color = instanceData.color;
}