#version 150
in vec2 position;
void main() {
    gl_position = vec4(position, 0.0, 1.0);
}