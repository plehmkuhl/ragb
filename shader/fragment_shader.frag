#version 430 core

struct bg_control_s {
    uint priority;
    uint character_base;
    bool mosaic;
    bool hi_color_palette;
    uint screen_base_block;
};

layout(std430, binding = 0) buffer display_ram 
{
    uint pram[1024];
    uint vram[98304];
    uint oam[1024];
};

layout(std430, binding = 1) buffer lcd_control {
    uint bg_mode;
    uint frame_select;
    bool one_dimensional_vram_mapping;
    bool forced_blank;
    bool display_bg0;
    bool display_bg1;
    bool display_bg2;
    bool display_bg3;
    bool display_window_0;
    bool display_window_1;
    bool display_obj;
};

layout(std430, binding = 2) buffer bg_control {
    bg_control_s bgctrl[4];
};

out vec4 out_color;
void main() {
    ivec2 screen;
    screen.x = int(gl_FragCoord.x);
    screen.y = int(160+68 - gl_FragCoord.y);

    if (screen.x == 241)
        out_color = vec4(0, 0, 1.0, 1.0);
    else if (screen.y == 161)
        out_color = vec4(0, 1, 0, 1.0);
    else if (screen.x > 241 || screen.y > 161)
        out_color = vec4(0.1, 0.1, 0.1, 1.0);
    else 
        out_color = vec4(gl_FragCoord.x / 512, gl_FragCoord.y / 512, 0.0, 1.0);
}