//! Initialize acquired window images before Bevy's camera passes.
//!
//! Bevy 0.19 startup can present a view before its output pass writes the
//! swapchain image (bevyengine/bevy#22733). A clear submission establishes the
//! image layout and consumes the acquisition semaphore even during warmup.
//! Keep validation enabled. This costs one small clear pass per acquired window.
use bevy::{
    prelude::*,
    render::{
        RenderApp,
        render_resource::{
            CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment,
            RenderPassDescriptor, StoreOp, TextureViewDescriptor,
        },
        renderer::{RenderDevice, RenderGraph, RenderGraphSystems, RenderQueue},
        view::window::ExtractedWindows,
    },
};

pub struct PresentationGuard;
impl Plugin for PresentationGuard {
    fn build(&self, _app: &mut App) {}
    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                RenderGraph,
                initialize_windows.in_set(RenderGraphSystems::Begin),
            );
        }
    }
}
fn initialize_windows(
    windows: Res<ExtractedWindows>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    let mut encoder = None;
    for window in windows.values() {
        let Some(surface) = window.swap_chain_texture.as_ref() else {
            continue;
        };
        // Use the actual surface, even if screenshots temporarily substitute
        // swap_chain_texture_view with an offscreen capture texture.
        let view = surface
            .texture
            .create_view(&TextureViewDescriptor::default());
        let encoder = encoder.get_or_insert_with(|| {
            device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("phalanx_window_initialization"),
            })
        });
        let attachments = [Some(RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::srgb(0.025, 0.035, 0.055).to_linear().into()),
                store: StoreOp::Store,
            },
        })];
        drop(encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("phalanx_window_initialization"),
            color_attachments: &attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        }));
    }
    if let Some(encoder) = encoder {
        queue.submit([encoder.finish()]);
    }
}

/// Keep backend checks, but omit optional native object names. Local heap
/// profiles showed Vulkan validation retaining names during paused rendering.
/// wgpu's own validation and Rust-side resource labels are unaffected.
/// WGPU_DISCARD_HAL_LABELS=0 restores native names for graphics debugging.
pub fn player_instance_flags(
    base: bevy::render::settings::InstanceFlags,
) -> bevy::render::settings::InstanceFlags {
    (base | bevy::render::settings::InstanceFlags::DISCARD_HAL_LABELS).with_env()
}
