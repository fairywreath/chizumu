#version 460 core

#pragma shader_stage(vertex)

layout(location = 0) in vec3 position;

layout(location = 0) out vec4 outColor;

struct PlatformInstanceData
{
    mat4 model;
};

layout(std140, binding = 0) uniform GlobalSceneUbo
{
    mat4 viewProj;
    vec2 viewport;
    mat4 runner;
};

layout(std430, binding = 1) readonly buffer PlatformInstanceDataSbo
{
    PlatformInstanceData platforms[];
};

void main()
{
    uint numVerticesPerInstances = gl_BaseInstance & 0xFFFF;
    uint ssboOffset = gl_BaseInstance >> 16;
    uint planeIndex = uint(floor(gl_VertexIndex / numVerticesPerInstances)) + ssboOffset;
    PlatformInstanceData platform = platforms[planeIndex];

    gl_Position = viewProj * runner * platform.model * vec4(position, 1.0);

    // XXX TODO: Get this value from the CPU.
    outColor = vec4(0.0, 0.0, 0.0, 1.0);
}