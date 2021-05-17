use std::sync::Arc;

use actor::Address;
use nalgebra::{RealField, Translation3, UnitQuaternion, Vector3};

use vulkan::{Device, PipelineCache, PipelineCacheCreateFlags, FALSE, TRUE};

use crate::{
    cull::CullContext,
    debug::DebugContext,
    depth::DepthContext,
    egui::EguiContext,
    error::Error,
    renderer::RenderConfiguration,
    scene::{Camera, MeshData, Model, Object, SceneContext, Sphere, Transform, Vertex},
    ApplicationMessage,
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
}

impl Context {
    pub fn new(
        address: Address<ApplicationMessage>,
        device: &Arc<Device>,
        configuration: RenderConfiguration,
        scale_factor: f32,
    ) -> Result<Self, Error> {
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
        let debug = DebugContext::new(address);
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
        for x in -20..21 {
            for y in -20..21 {
                for z in -20..21 {
                    scene.insert_object(Object {
                        model: cube_model,
                        transform: Transform {
                            translation: Vector3::new(
                                x as f32 * 3.0,
                                y as f32 * 3.0,
                                z as f32 * 3.0,
                            ),
                            scale: Vector3::new(0.3, 0.3, 0.3),
                            rotation: UnitQuaternion::from_euler_angles(
                                x as f32, y as f32, z as f32,
                            ),
                        },
                    });
                }
            }
        }
        scene.flush()?;
        let depth = DepthContext::new(device, &pipeline_cache)?;
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
        })
    }
}
