const WINDOW_NAME: &str = "WGPU Shader Playground";
const WINDOW_INITIAL_WIDTH: u32 = 1280;
const WINDOW_INITIAL_HEIGHT: u32 = 720;

use notify::Watcher;
// === === === === === === === === === === === === === === === ===
use wgpu::{
    include_wgsl, util::DeviceExt, Buffer, Color, Device, Queue, RenderPipeline, ShaderModule,
    Surface, SurfaceConfiguration,
};
use winit::dpi::PhysicalSize;

#[rustfmt::skip]
const QUAD_VERTEX: [f32; 8] = [
    -1.0,  1.0,
    -1.0, -1.0,
     1.0, -1.0,
     1.0,  1.0,
];

#[rustfmt::skip]
const QUAD_INDICES: [u32; 6] = [
    0, 1, 2,
    0, 2, 3,
];

struct Gpu {
    device: Device,
    queue: Queue,
    surface: Surface,
    surface_config: SurfaceConfiguration,

    quad_vertex: Buffer,
    quad_indices: Buffer,
    pipeline: RenderPipeline,

    clear_color: Color,
}

impl Gpu {
    pub async fn new(window: &winit::window::Window) -> anyhow::Result<Self> {
        let dimensions = window.inner_size();

        let instance = wgpu::Instance::default();
        let surface = unsafe { instance.create_surface(window)? };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;

        device.on_uncaptured_error(Box::new(|e| match e {
            wgpu::Error::Validation { description, .. } => {
                log::error!("Validation Error: {description}")
            }
            _ => panic!("{}", e),
        }));

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_config = SurfaceConfiguration {
            alpha_mode: surface_caps.alpha_modes[0],
            format: surface_caps.formats[0],
            height: dimensions.height,
            width: dimensions.width,
            present_mode: wgpu::PresentMode::Fifo, // VSYNC will be fine for experimenting
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        let quad_vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&QUAD_VERTEX),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let quad_indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let fragment_shader = device.create_shader_module(include_wgsl!("../main.wgsl"));
        let pipeline = Self::init_pipeline(&device, &surface_config, &fragment_shader);

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,

            quad_vertex,
            quad_indices,
            pipeline,

            clear_color: Color::BLACK,
        })
    }

    fn quad_vertex_desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            }],
        }
    }

    fn init_pipeline(
        device: &Device,
        surface_config: &SurfaceConfiguration,
        fragment: &ShaderModule,
    ) -> RenderPipeline {
        let shader = device.create_shader_module(include_wgsl!("./vertex.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            primitive: wgpu::PrimitiveState {
                conservative: false,
                cull_mode: None,
                front_face: wgpu::FrontFace::Ccw,
                strip_index_format: None,
                unclipped_depth: false,
                topology: wgpu::PrimitiveTopology::TriangleList,
                polygon_mode: wgpu::PolygonMode::Fill,
            },
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Self::quad_vertex_desc()],
            },
            fragment: Some(wgpu::FragmentState {
                entry_point: "fs_main",
                module: fragment,
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
        })
    }

    pub fn reload_fragment_shader(&mut self, src: &str) {
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(src.into()),
            });

        let pipeline = Self::init_pipeline(&self.device, &self.surface_config, &shader);
        self.pipeline = pipeline;
    }

    pub fn resize_surface(&mut self, dimensions: &PhysicalSize<u32>) {
        self.surface_config.width = dimensions.width;
        self.surface_config.height = dimensions.height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn render(&self) -> anyhow::Result<()> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut p = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    resolve_target: None,
                    view: &view,
                    ops: wgpu::Operations {
                        store: true,
                        load: wgpu::LoadOp::Clear(self.clear_color),
                    },
                })],
                depth_stencil_attachment: None,
            });

            p.set_pipeline(&self.pipeline);
            p.set_vertex_buffer(0, self.quad_vertex.slice(..));
            p.set_index_buffer(self.quad_indices.slice(..), wgpu::IndexFormat::Uint32);

            p.draw_indexed(0..QUAD_INDICES.len() as u32, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

// === === === === === === === === === === === === === === === ===
fn init_window() -> anyhow::Result<(winit::window::Window, winit::event_loop::EventLoop<()>)> {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title(WINDOW_NAME)
        .with_inner_size(winit::dpi::PhysicalSize::new(
            WINDOW_INITIAL_WIDTH,
            WINDOW_INITIAL_HEIGHT,
        ))
        .build(&event_loop)?;

    Ok((window, event_loop))
}

fn shader_watcher(
    path: &str,
    tx: std::sync::mpsc::Sender<String>,
) -> notify::Result<notify::RecommendedWatcher> {
    use notify::{
        event::{DataChange, ModifyKind},
        EventKind,
    };

    let path = path.to_owned();

    notify::recommended_watcher(move |res| {
        if let Err(e) = res {
            log::error!("Something went wrong reading notification event: {e}");
            return;
        }

        let res: notify::Event = res.unwrap();
        match res.kind {
            EventKind::Modify(ModifyKind::Data(DataChange::Content)) => {
                let shader_src = std::fs::read_to_string(&path.clone()).unwrap();
                tx.send(shader_src).unwrap();
            }
            _ => (),
        };
    })
}

fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .with_module_level("wgpu_core", log::LevelFilter::Warn)
        .with_module_level("wgpu_hal", log::LevelFilter::Warn)
        .init()?;

    let (window, event_loop) = init_window()?;
    let mut gpu = pollster::block_on(Gpu::new(&window))?;

    let (tx, rx) = std::sync::mpsc::channel::<String>();

    let mut watcher = shader_watcher("./main.wgsl", tx)?;
    watcher.watch(
        std::path::Path::new("./main.wgsl"),
        notify::RecursiveMode::NonRecursive,
    )?;

    use winit::{event::Event, event::WindowEvent};
    event_loop.run(move |event, _, control_flow| {
        if let Ok(src) = rx.try_recv() {
            gpu.reload_fragment_shader(&src);
        }

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = winit::event_loop::ControlFlow::Exit,

                WindowEvent::Resized(ref dimensions) => gpu.resize_surface(dimensions),
                WindowEvent::ScaleFactorChanged {
                    new_inner_size: dimensions,
                    ..
                } => gpu.resize_surface(dimensions), // resized

                _event => (), // everything else
            },
            Event::RedrawRequested(_) => {
                let _ = gpu.render();
            } // render
            Event::MainEventsCleared => window.request_redraw(),
            _ => (),
        }
    });
}
