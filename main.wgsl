// UNIFORMS =======================================================
@group(0) @binding(0)
var<uniform> mouse_pos: vec2<f32>;
@group(0) @binding(1)
var<uniform> window_dim: vec2<u32>;

fn window_dimensions() -> vec2<f32> {
    return vec2<f32>(window_dim);
}

// CONSTANTS ===========================================================
const MAX_RENDER_DIST: f32 = 6.0;
const MAX_STEPS: u32       = 1000u;
const MAX_DIST: f32        = 1000.0;
const SURFACE_DIST: f32    = .01;

const BASE_COLOR: vec4<f32>  = vec4<f32>(0.1, 0.1, 0.6, 1.0);
const MOUSE_COLOR: vec4<f32> = vec4<f32>(1.0, 1.0, 1.0, 0.8);

// DISTANCE FUNCTIONS ==================================================
fn sphere_dist(p: vec3<f32>, sphere: vec4<f32>) -> f32 {
    return length(p - sphere.xyz) - sphere.w;
}

fn torus_dist(p: vec3<f32>, o: vec3<f32>, r: vec2<f32>) -> f32 {
    let origin = p - o;
    let x = length(origin.xz) - r.x;
    return length(vec2<f32>(x, origin.y)) - r.y;
}

fn capsule_dist(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> f32 {
    let ab = b - a;
    let ap = p - a;
    var t = dot(ab, ap) / dot(ab, ab);
    t = clamp(t, 0.0, 1.0);

    let c = a + t * ab;

    return length(p - c) - r;
}

fn cube_dist(p: vec3<f32>, o: vec3<f32>, s: f32) -> f32 {
    let origin = p - o;
    return length(max(abs(origin) - vec3<f32>(s), vec3<f32>(0.0)));
}

fn plane_dist(p: vec3<f32>) -> f32 {
    return p.y + 1.0;
}

// RAY-MARCH FUNCTIONS =================================================
fn smooth_union(d1: f32, d2: f32, k: f32) -> f32 {
    let h = max(k - abs(d1-d2), 0.0);
    return min(d1, d2) - h * h * 0.25 / k;
}

fn get_dist(p: vec3<f32>) -> f32 {
    let sphere = sphere_dist(p, vec4<f32>(-5.0, 0.0, 4.5, 1.0));

    let a = vec3<f32>(-3.0, -0.7, 4.5);
    let b = vec3<f32>(-3.0, 1.0, 4.5);
    let capsule = capsule_dist(p, a, b, 0.5);

    let t = vec3<f32>(-1.0, -0.8, 5.0);
    let tr = vec2<f32>(1.0, 0.3);
    let torus = torus_dist(p, t, tr);

    let box = cube_dist(p, vec3<f32>(2.0, 0.0, 5.0), 1.0);

    let plane = plane_dist(p);

    var d = min(capsule, sphere);
    d = min(d, torus);
    d = min(d, box);
    return smooth_union(d, plane, 0.5);
}

fn raymarch(ro: vec3<f32>, rd: vec3<f32>) -> f32 {
    var dO = 0.0; // Distance from origin

    for (var i: u32 = 0u; i < MAX_STEPS; i++) {
        let p: vec3<f32> = ro + dO * rd;

        let ds = get_dist(p);
        dO += ds;

        if ds < SURFACE_DIST || dO > MAX_DIST {
            break;
        }
    }
    return dO;
}

// LIGHT ===================================================
fn get_normal(p: vec3<f32>) -> vec3<f32> {
    let d = get_dist(p);
    let e = vec2<f32>(0.01, 0.0);

    let n = d - vec3<f32>(
        get_dist(p - e.xyy),
        get_dist(p - e.yxy),
        get_dist(p - e.yyx),
    );
    return normalize(n);
}

fn get_light(p: vec3<f32>) -> f32 {
    var light_pos = vec3<f32>(-1.0, 5.0, 2.0);
    let mp = (window_dimensions().y - mouse_pos.y);

    light_pos.y =  clamp(((2.0 * mp/f32(window_dim.y)) - 1.0) * 20.0, 0.0, 2000.0);
    light_pos.x = ((2.0 * mouse_pos.x/f32(window_dim.x)) - 1.0) * 10.0;

    let l = normalize(light_pos - p);
    let n = get_normal(p);

    var dif = clamp(dot(n, l), 0.0, 1.0);
    let d = raymarch(p + n * SURFACE_DIST * 2.0, l);
    if d < length(light_pos - p) {
        dif *= .3;
    }
    return dif;
}

@fragment
fn fs_main(@builtin(position) coord_in: vec4<f32>) -> @location(0) vec4<f32> {
    // Mouse Control =======================================
    let mouse_radius: f32 = 0.02 * f32(window_dim.x);
    if distance(coord_in.xy, mouse_pos) <= mouse_radius {
        return MOUSE_COLOR;
    }

    // Normalization =======================================
    let resolution = window_dimensions();
    // Raymarching algorithms works better with glsl-like coordinates
    let coord = vec2<f32>(coord_in.x, resolution.y - coord_in.y);

    let uv = (2.0 * coord.xy - resolution)/resolution.y;
    var col = vec3<f32>(0.0);

    let ro = vec3<f32>(0.0, 1.0, 0.0);
    let rd = normalize(vec3<f32>(uv.x, uv.y, 0.5));
    var d = raymarch(ro, rd);

    let p = ro + rd * d;

    let dif = get_light(p);
    col = vec3<f32>(dif/4.0, dif/2.0, dif);

    let n = get_normal(p);
    let bounce_light = clamp(0.5 + 0.5 * dot(n, vec3<f32>(0.0, -1.0, 0.0)), 0.0, 1.0);
    col += vec3<f32>(0.1, 0.1, 0.3) * bounce_light;

    {
        var dif = sqrt(clamp(0.5 + 0.5 * n.y, 0.0, 1.0 ));
        let back = 0.2 + 0.2 * sin(2.0 + vec3<f32>(0.0,1.0,2.0));

        let refl = reflect(rd, n);

        var spe = smoothstep(-0.2, 0.2, refl.y);
        spe *= dif;
        spe *= 0.04 + 0.5 * pow(clamp(1.0+dot(n,rd),0.0,1.0), 5.0 );

        col += back * 0.60 * dif * vec3<f32>(0.3, 0.60, 8.15);
        col += 2.00 * spe * vec3(0.40,0.60,1.30) * 0.4;
    }

    return vec4<f32>(col, 1.0);
}
