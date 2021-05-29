use std::sync::Arc;

use eyre::Context as EyreContext;
use nalgebra::{RealField, Translation3, Vector3};

use vulkan::{
    CommandBuffer, CommandBufferLevel, CommandPool, CommandPoolCreateFlags, DescriptorPool,
    DescriptorPoolSetup, Device, PipelineCache, PipelineCacheCreateFlags, FALSE, TRUE,
};

use crate::{
    cull::CullContext,
    debug::DebugContext,
    depth::DepthContext,
    egui::EguiContext,
    frame::Frame,
    renderer::RenderConfiguration,
    scene::{Camera, MeshData, Model, SceneContext, Sphere, Vertex},
};

pub struct Context {
    pub cull: CullContext,
    pub depth: DepthContext,
    pub scene: SceneContext,
    pub cube_model: u32,
    pub pipeline_cache: PipelineCache,
    pub configuration: RenderConfiguration,
    pub egui: EguiContext,
    pub debug: DebugContext,
    pub descriptor_pool: DescriptorPool,
    pub command_pool: CommandPool,
    pub command_buffer: CommandBuffer,
}

impl Context {
    pub fn new(
        device: &Arc<Device>,
        configuration: RenderConfiguration,
        scale_factor: f32,
    ) -> eyre::Result<Self> {
        let pipeline_cache =
            device.create_pipeline_cache(PipelineCacheCreateFlags::empty(), None)?;
        let camera = Camera {
            translation: Translation3::new(0.0, 0.0, 0.0),
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
            fovy: f32::pi() / 3.0,
            znear: 0.1,
            zfar: 1000.0,
        };
        let mut scene = SceneContext::new(device, 128, 128, 128, camera)?;
        let egui = EguiContext::new(device, scale_factor)?;
        let debug = DebugContext::default();
        let cube = MeshData {
            vertices: vec![
                Vertex {
                    pos: Vector3::new(-1.0, -1.0, -1.0),
                    color: Vector3::new(0.0, 0.0, 0.0),
                },
                Vertex {
                    pos: Vector3::new(-1.0, -1.0, 1.0),
                    color: Vector3::new(0.0, 0.0, 1.0),
                },
                Vertex {
                    pos: Vector3::new(-1.0, 1.0, -1.0),
                    color: Vector3::new(0.0, 1.0, 0.0),
                },
                Vertex {
                    pos: Vector3::new(-1.0, 1.0, 1.0),
                    color: Vector3::new(0.0, 1.0, 1.0),
                },
                Vertex {
                    pos: Vector3::new(1.0, -1.0, -1.0),
                    color: Vector3::new(1.0, 0.0, 0.0),
                },
                Vertex {
                    pos: Vector3::new(1.0, -1.0, 1.0),
                    color: Vector3::new(1.0, 0.0, 1.0),
                },
                Vertex {
                    pos: Vector3::new(1.0, 1.0, -1.0),
                    color: Vector3::new(1.0, 1.0, 0.0),
                },
                Vertex {
                    pos: Vector3::new(1.0, 1.0, 1.0),
                    color: Vector3::new(1.0, 1.0, 1.0),
                },
            ],
            indices: vec![
                0, 1, 3, 0, 3, 2, 0, 2, 4, 2, 6, 4, 0, 4, 5, 0, 5, 1, 1, 5, 7, 1, 7, 3, 2, 3, 6, 3,
                7, 6, 4, 6, 5, 5, 6, 7,
            ],
        };
        let cube_mesh = scene.insert_mesh(cube);
        let cube_model = scene.insert_model(Model {
            bounds: Sphere {
                center: Vector3::new(1.0, 1.0, 1.0),
                radius: 3f32.sqrt(),
            },
            mesh: cube_mesh,
        });
        scene.flush().wrap_err("could not flush scene buffers")?;
        let command_pool = device.create_command_pool(
            CommandPoolCreateFlags::TRANSIENT,
            device.main_queue_family_index(),
        )?;
        let mut command_buffers = command_pool.allocate(CommandBufferLevel::PRIMARY, 1)?;
        let command_buffer = command_buffers.remove(0);
        let descriptor_pool =
            (Context::pool_setup() + Frame::pool_setup() * 2).create_pool(device)?;
        let depth = DepthContext::new(device, &pipeline_cache, &descriptor_pool)?;
        let cull = CullContext::new(
            device,
            if configuration.use_draw_count {
                TRUE
            } else {
                FALSE
            },
            &pipeline_cache,
        )?;
        Ok(Self {
            cull,
            depth,
            scene,
            cube_model,
            pipeline_cache,
            configuration,
            egui,
            debug,
            descriptor_pool,
            command_pool,
            command_buffer,
        })
    }

    pub fn pool_setup() -> DescriptorPoolSetup {
        DepthContext::pool_setup()
    }
}
