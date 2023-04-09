@group(0) @binding(0)
var<uniform> mouse_pos: vec2<f32>;
@group(0) @binding(1)
var<uniform> window_dim: vec2<u32>;


@fragment
fn fs_main(@builtin(position) coord_in: vec4<f32>) -> @location(0) vec4<f32> {
    let radius = 0.1 * f32(window_dim.x);

    var end_color = vec4<f32>(0.1, 0.1, 0.6, 1.0);

    if distance(coord_in.xy, mouse_pos) < radius {
        end_color = end_color * vec4<f32>(0.2, 0.2, 0.2, 1.0);
    }
    if coord_in.x < f32(window_dim.x) / 2.0 {
        end_color = end_color * vec4<f32>(0.2, 0.2, 0.2, 1.0);
    }
    return end_color;
}
