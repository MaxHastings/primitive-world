use bytemuck::{Pod, Zeroable};

use crate::simulation::{MAX_AGENTS, Simulation, WORLD_SIZE};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub center: [f32; 2],
    pub zoom: f32,
    pub aspect: f32,
    pub lens: u32,
    pub point_size: f32,
    pub selected_id: u32,
    pub _padding: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lens {
    Normal = 0,
    ResourceDensity = 1,
    AgentDensity = 2,
    Energy = 3,
    Movement = 4,
    Age = 5,
    Attention = 6,
    CarriedFood = 7,
    Action = 8,
    Fertility = 9,
}

impl Lens {
    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::ResourceDensity => "Resource density",
            Self::AgentDensity => "Agent density",
            Self::Energy => "Energy",
            Self::Movement => "Movement",
            Self::Age => "Age",
            Self::Attention => "Sensor orientation",
            Self::CarriedFood => "Carried food",
            Self::Action => "Current action",
            Self::Fertility => "Landscape fertility",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Normal => Self::ResourceDensity,
            Self::ResourceDensity => Self::AgentDensity,
            Self::AgentDensity => Self::Energy,
            Self::Energy => Self::Movement,
            Self::Movement => Self::Age,
            Self::Age => Self::Attention,
            Self::Attention => Self::CarriedFood,
            Self::CarriedFood => Self::Action,
            Self::Action => Self::Fertility,
            Self::Fertility => Self::Normal,
        }
    }
}

pub struct Renderer {
    pub camera: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    world_pipeline: wgpu::RenderPipeline,
    world_bind_group: wgpu::BindGroup,
    agent_pipeline: wgpu::RenderPipeline,
    agent_bind_groups: [wgpu::BindGroup; 2],
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        simulation: &Simulation,
        width: u32,
        height: u32,
    ) -> Self {
        let camera = CameraUniform {
            center: [WORLD_SIZE * 0.5, WORLD_SIZE * 0.5],
            zoom: 1.0,
            aspect: width.max(1) as f32 / height.max(1) as f32,
            lens: Lens::Energy as u32,
            point_size: 2.0,
            selected_id: u32::MAX,
            _padding: 0,
        };
        let camera_buffer = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("camera uniform"),
                contents: bytemuck::bytes_of(&camera),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        );

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bind group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let world_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("world render layout"),
            entries: &[
                storage_entry(3, wgpu::ShaderStages::FRAGMENT, true),
                storage_entry(0, wgpu::ShaderStages::FRAGMENT, true),
                uniform_entry(1, wgpu::ShaderStages::FRAGMENT),
                storage_entry(2, wgpu::ShaderStages::FRAGMENT, true),
            ],
        });
        let world_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("world render bind group"),
            layout: &world_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: simulation.ground_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: simulation.resource_display_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: simulation.params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: simulation.occupancy_buffer.as_entire_binding(),
                },
            ],
        });
        let agent_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("agent render layout"),
            entries: &[
                storage_entry(0, wgpu::ShaderStages::VERTEX, true),
                storage_entry(1, wgpu::ShaderStages::VERTEX, true),
                storage_entry(2, wgpu::ShaderStages::VERTEX, true),
            ],
        });
        let agent_bind_groups = [
            make_agent_group(
                device,
                &agent_layout,
                &simulation.agent_buffers[0],
                &simulation.perception_buffer,
                &simulation.occupancy_buffer,
            ),
            make_agent_group(
                device,
                &agent_layout,
                &simulation.agent_buffers[1],
                &simulation.perception_buffer,
                &simulation.occupancy_buffer,
            ),
        ];

        let world_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("world render shader"),
            source: wgpu::ShaderSource::Wgsl(
                crate::simulation::shader_source(include_str!("../shaders/render_world.wgsl"))
                    .into(),
            ),
        });
        let agent_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("agent render shader"),
            source: wgpu::ShaderSource::Wgsl(
                crate::simulation::shader_source(include_str!("../shaders/render_agents.wgsl"))
                    .into(),
            ),
        });
        let world_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("world pipeline layout"),
                bind_group_layouts: &[&camera_layout, &world_layout],
                push_constant_ranges: &[],
            });
        let world_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("resource field pipeline"),
            layout: Some(&world_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &world_shader,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &world_shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let agent_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("agent pipeline layout"),
                bind_group_layouts: &[&camera_layout, &agent_layout],
                push_constant_ranges: &[],
            });
        let agent_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("agent point pipeline"),
            layout: Some(&agent_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &agent_shader,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &agent_shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            camera,
            camera_buffer,
            camera_bind_group,
            world_pipeline,
            world_bind_group,
            agent_pipeline,
            agent_bind_groups,
        }
    }

    pub fn update_camera(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&self.camera));
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, simulation: &'a Simulation) {
        pass.set_pipeline(&self.world_pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(1, &self.world_bind_group, &[]);
        pass.draw(0..3, 0..1);
        pass.set_pipeline(&self.agent_pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(1, &self.agent_bind_groups[simulation.current_buffer], &[]);
        pass.draw(0..6, 0..MAX_AGENTS);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.camera.aspect = width.max(1) as f32 / height.max(1) as f32;
    }
}

fn storage_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    read_only: bool,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn make_agent_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    agents: &wgpu::Buffer,
    perceptions: &wgpu::Buffer,
    occupancy: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("agent render bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: agents.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: perceptions.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: occupancy.as_entire_binding(),
            },
        ],
    })
}
