#version 460 core

#pragma shader_stage(fragment)

layout(location = 0) in vec4 inColor;
layout(location = 1) in float thickness;
layout(location = 2) in vec2 smoothOffsets;

layout(location = 0) out vec4 fragColor;

const float aaRadius = 1.5;

void main()
{
    vec4 color = inColor;

    float w = thickness / 2.0 - aaRadius;
    float d = abs(smoothOffsets.x) - w;
    if (d >= 0)
    {
        d /= aaRadius;
        color.a *= exp(-d * d);
    }

    // XXX: Make this customizable from CPU.
    // Additional smoothing/fading on line endings(?)
    // float lengthD = smoothstep(0.0, 0.1, smoothOffsets.y);
    // color.a = min(lengthD, color.a * lengthD * 1.1);

    fragColor = color;
}
