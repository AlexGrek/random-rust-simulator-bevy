#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0)
var color_texture: texture_2d<f32>;

@group(2) @binding(1)
var color_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(color_texture, color_sampler, in.uv);
    return tex_color; // just output sampled texture
}
