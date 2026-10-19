pub const VERTEX: &str = r#"#version 330 core
layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_local;
layout(location = 2) in vec4 a_shape;
layout(location = 3) in vec4 a_color;
layout(location = 4) in vec4 a_params;
layout(location = 5) in vec4 a_extra;
uniform vec2 u_viewport;
out vec2 v_local;
out vec2 v_pixel;
flat out vec4 v_shape;
out vec4 v_color;
flat out vec4 v_params;
out vec4 v_extra;
void main() {
    v_local = a_local;
    v_pixel = a_pos;
    v_shape = a_shape;
    v_color = a_color;
    v_params = a_params;
    v_extra = a_extra;
    vec2 ndc = a_pos / u_viewport * 2.0 - 1.0;
    gl_Position = vec4(ndc.x, -ndc.y, 0.0, 1.0);
}
"#;

pub const FRAGMENT: &str = r#"#version 330 core
in vec2 v_local;
in vec2 v_pixel;
flat in vec4 v_shape;
in vec4 v_color;
flat in vec4 v_params;
in vec4 v_extra;
uniform sampler2D u_atlas;
uniform sampler2D u_images;
uniform vec4 u_clip;
uniform vec2 u_viewport;
out vec4 o_color;

const float PI = 3.14159265358979;

float sd_round(vec2 p, vec2 h, float r) {
    r = min(r, min(h.x, h.y));
    vec2 q = abs(p) - h + r;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

float coverage(float d) {
    return clamp(0.5 - d, 0.0, 1.0);
}

vec2 erf2(vec2 x) {
    vec2 s = sign(x);
    vec2 a = abs(x);
    x = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
    x *= x;
    return s - s / (x * x);
}

float gaussian(float x, float sigma) {
    return exp(-(x * x) / (2.0 * sigma * sigma)) / (sqrt(2.0 * PI) * sigma);
}

float shadow_x(float x, float y, float sigma, float corner, vec2 h) {
    float delta = min(h.y - corner - abs(y), 0.0);
    float curved = h.x - corner + sqrt(max(0.0, corner * corner - delta * delta));
    vec2 integral = 0.5 + 0.5 * erf2((x + vec2(-curved, curved)) * (sqrt(0.5) / sigma));
    return integral.y - integral.x;
}

float shadow(vec2 p, vec2 h, float corner, float sigma) {
    corner = min(corner, min(h.x, h.y));
    if (sigma < 0.01) {
        return coverage(sd_round(p, h, corner));
    }
    float low = p.y - h.y;
    float high = p.y + h.y;
    float start = clamp(-3.0 * sigma, low, high);
    float end = clamp(3.0 * sigma, low, high);
    float step = (end - start) / 4.0;
    float y = start + step * 0.5;
    float value = 0.0;
    for (int i = 0; i < 4; i++) {
        value += shadow_x(p.x, p.y - y, sigma, corner, h) * gaussian(y, sigma) * step;
        y += step;
    }
    return value;
}

float window_mask() {
    vec2 p = v_pixel - u_clip.xy;
    return coverage(sd_round(p, u_clip.xy, u_clip.z));
}

void main() {
    int kind = int(v_params.x + 0.5);
    vec2 h = v_shape.xy;
    float r = v_shape.z;
    float alpha = 0.0;
    if (kind == 0) {
        alpha = coverage(sd_round(v_local, h, r));
    } else if (kind == 1) {
        float d = sd_round(v_local, h, r);
        alpha = coverage(d - v_params.z) - coverage(d - v_params.y);
    } else if (kind == 2) {
        alpha = shadow(v_local, h, r, v_params.y);
        if (v_params.z > 0.5) {
            alpha *= 1.0 - coverage(sd_round(v_local - v_extra.zw, v_extra.xy, v_shape.w));
        }
    } else if (kind == 3) {
        float m = texture(u_atlas, v_extra.xy).r;
        alpha = v_params.z > 0.5 ? 1.0 - pow(1.0 - m, v_params.y) : pow(m, v_params.y);
    } else if (kind == 4) {
        float d = length(v_local / v_params.yz);
        alpha = 1.0 - clamp(d / v_params.w, 0.0, 1.0);
    } else if (kind == 5) {
        float radius = h.x;
        float width = v_params.y;
        float dist = length(v_local);
        float ring = coverage(dist - radius) - coverage(dist - (radius - width));
        float angle = atan(v_local.x, -v_local.y) - v_params.z;
        angle = mod(angle + PI, 2.0 * PI) - PI;
        float edge = (PI * 0.25 - abs(angle)) * max(dist, 1.0);
        float head = clamp(edge + 0.5, 0.0, 1.0);
        vec4 c = mix(v_extra, v_color, head);
        float a = ring * c.a * window_mask();
        o_color = vec4(c.rgb * a, a);
        return;
    } else if (kind == 6) {
        float m = coverage(sd_round(v_local, h, r)) * window_mask();
        o_color = texture(u_images, v_extra.xy) * m;
        return;
    }
    float a = alpha * v_color.a * window_mask();
    o_color = vec4(v_color.rgb * a, a);
}
"#;
