#version 330 core

layout(location = 0) in vec2 in_position;

out vec2 UV;

void main()
{
    gl_Position.xy = in_position;
    gl_Position.z = 0.0;
    gl_Position.w = 1.0;

    UV.x = (in_position.x + 1.0) / 2.0;
    UV.y = (in_position.y + 1.0) / 2.0;
}