#version 460 core

#pragma shader_stage(vertex)

layout(location = 0) out vec4 outColor;

/* Vertex attribute data combined with transform data as most planes will only be drawn "once". */
struct SimplePlaneData
{
    vec4 vertices[4];
    mat4 model;
    vec4 color;
};

layout(std140, binding = 0) uniform GlobalSceneUbo
{
    mat4 viewProj;
    vec2 viewport;
    mat4 runner;
};

layout(std430, binding = 1) readonly buffer SimplePlaneDataSbo
{
    SimplePlaneData planes[];
};

const uint quadIndices[6] = {0, 1, 2, 1, 2, 3};

void main()
{
    uint planeIndex = uint(floor(gl_VertexIndex / 6));
    SimplePlaneData plane = planes[planeIndex];

    vec3 position = plane.vertices[quadIndices[gl_VertexIndex % 6]].xyz;

    gl_Position = viewProj * runner * plane.model * vec4(position, 1.0);
    outColor = plane.color;
}