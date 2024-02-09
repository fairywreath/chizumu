#version 460 core

#pragma shader_stage(vertex)

layout(location = 0) out vec4 outColor;
layout(location = 1) out float outThickness;
layout(location = 2) out vec2 smoothOffsets;

struct LineData
{
    // Line primitive data.
    vec3 p0;
    vec3 p1;
    float thickness;
    vec4 color;

    // Transform data.
    mat4 model;
};

layout(std140, binding = 0) uniform GlobalSceneUbo
{
    mat4 viewProj;
    uvec2 viewport;
};

layout(std430, binding = 1) readonly buffer LineDataSbo
{
    LineData lines[];
};

const float aaRadius = 1.5;
const vec2 quad[6] = vec2[6](vec2(0, -1), vec2(0, 1), vec2(1, -1), vec2(0, 1), vec2(1, -1), vec2(1, 1));

void main()
{
    uint currentLineIndex = uint(floor(gl_VertexIndex / 6));
    LineData line = lines[currentLineIndex];

    vec3 p0 = line.p0.xyz;
    vec3 p1 = line.p1.xyz;
    float thickness = line.thickness;

    mat4 mvp = viewProj * line.model;

    /* Transform line to a quad.*/
    vec4 clip0 = mvp * vec4(p0, 1.0);
    vec4 clip1 = mvp * vec4(p1, 1.0);
    vec2 screen0 = viewport * ((clip0.xy / clip0.w) + 1.0) / 2.0;
    vec2 screen1 = viewport * ((clip1.xy / clip1.w) + 1.0) / 2.0;

    float width = thickness / 2.0 + aaRadius;
    vec2 direction = normalize(screen1.xy - screen0.xy);
    vec2 normal = vec2(-direction.y, direction.x);

    int quadIndex = gl_VertexIndex % 6;
    vec2 quadVertex = quad[gl_VertexIndex % 6];
    vec2 screenOriginal = screen0 * quadVertex.x + screen1 * (1.0 - quadVertex.x);
    vec4 clipCurrent = clip0 * quadVertex.x + clip1 * (1.0 - quadVertex.x);

    /* Extend width.*/
    float widthExtendAmount = quadVertex.y * width;
    vec2 screenNew = screenOriginal + (normal * widthExtendAmount);

    gl_Position = vec4(2.0 * screenNew / viewport - 1.0, clipCurrent.z / clipCurrent.w, 1.0);

    outColor = line.color;
    outThickness = thickness;
    smoothOffsets = vec2(widthExtendAmount, float(quadVertex.x));
}