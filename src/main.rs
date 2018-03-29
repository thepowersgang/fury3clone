extern crate amethyst;
extern crate byteorder;
#[macro_use]
extern crate log;
extern crate env_logger;

use amethyst::prelude::*;
use amethyst::renderer::Rgba;
use amethyst::renderer::Event;
use amethyst::core::transform::Transform;
use amethyst::core::cgmath::Deg;
use amethyst::core::cgmath::Vector3;
use amethyst::ecs;

use amethyst::renderer as a_renderer;

mod datafile;

struct GameRoot
{
    data_pod: self::datafile::PodArchive,
}

fn main()
{
    env_logger::init();
    main_res().unwrap();
}
fn main_res() -> Result<(), Box<::std::error::Error>>
{
    let pipe = ::amethyst::renderer::Pipeline::build().with_stage(
        ::amethyst::renderer::Stage::with_backbuffer()
            .clear_target(Rgba(0.,0.,0.,0.), 1.0)
            .with_pass(::amethyst::renderer::DrawShadedSeparate::new()),
        );
    
    let key_bindings_path = "resources/input.ron";
    let display_config_path = "resources/fgl.ron";

    let config = ::amethyst::renderer::DisplayConfig::load(display_config_path);

    let root = GameRoot {
        data_pod: self::datafile::PodArchive::from_file(r"V:\Games\Fury3\SYSTEM\FURY3.POD")?,
        };
    let mut game = Application::build("resources/assets", root)?
        .with_bundle(
            ::amethyst::input::InputBundle::<String, String>::new().with_bindings_from_file(&key_bindings_path),
            )?
        .with_bundle(::amethyst::renderer::RenderBundle::new())?
        .with_local(::amethyst::renderer::RenderSystem::build(pipe, Some(config))?)
        .with(CameraMoveSystem::new(), "camera", &[])
        .build()?;
    game.run();
    Ok(())
}

impl State for GameRoot
{
    fn on_start(&mut self, world: &mut World)
    {
        // Add the first level as a model.
        let m = datafile::Model::from_bin_file( self.data_pod.open_file("MODELS\\EGYPT.BIN").unwrap() ).expect("Datafile load");
        
        let vertices_as_arrays: Vec<_> = m.faces.iter().flat_map(|v| v.v.iter().map(|&v| m.vertices[v])).collect();
        debug!("vertices_as_arrays.len() = {}", vertices_as_arrays.len());
        let normals: Vec<_> = m.faces.iter()
            .flat_map(|v| {
                let n = v.normal;
                v.v.iter().map(move |_| a_renderer::Separate::<a_renderer::Normal>::new(Vector3::from(n).into()))
                })
            .collect();
        let tex_coords: Vec<_> = m.faces.iter()
            .flat_map(|v| {
                v.v.iter().map(move |_| a_renderer::Separate::<a_renderer::TexCoord>::new([0.1,0.1]))
                })
            .collect();

        let (mesh, material) = {
            let loader = world.read_resource::<::amethyst::assets::Loader>();
            let m2: a_renderer::ComboMeshCreator = (
                vertices_as_arrays.into_iter().map(|p| a_renderer::Separate::<a_renderer::Position>::new(p)).collect::<Vec<_>>(),
                None,   // TODO: Colours
                Some(tex_coords),   // Texture coords (needed)
                Some(normals),   // TODO: Normals
                None,   // TODO: Tangents
                ).into();
            let mesh: ::amethyst::assets::Handle<a_renderer::Mesh> = loader.load_from_data(m2.into(), (), &world.read_resource());

            // Colour/material
            let tex_storage = world.read_resource();
            let mat_defaults = world.read_resource::<a_renderer::MaterialDefaults>();

            let albedo = [0.0, 0.0, 1.0, 1.0].into();
            let albedo = loader.load_from_data(albedo, (), &tex_storage);
            let mat = a_renderer::Material {
                albedo,
                ..mat_defaults.0.clone()
                };

            (mesh, mat)
            };

        world
            .create_entity()
            .with(Transform::default())
            .with(mesh)
            .with(material)
            .build()
            ;

            
        initialise_lights(world);
        initialise_camera(world);
    }
    fn handle_event(&mut self, _: &mut World, event: Event) -> Trans
    {
        macro_rules! key_input
        {
            ($keycode:ident) => (::amethyst::renderer::WindowEvent::KeyboardInput {
                    input: ::amethyst::renderer::KeyboardInput { virtual_keycode: Some(::amethyst::renderer::VirtualKeyCode::$keycode), .. },
                    ..
                    });
        }
        match event
        {
            Event::WindowEvent { event, .. } => match event {
                a_renderer::WindowEvent::Closed => Trans::Quit,
                key_input!(Escape) => Trans::Quit,
                //key_input!(A) => {
                //    //world.get
                //    },
                _ => Trans::None,
            },
            _ => Trans::None,
        }
    }
}


/// This function adds an ambient light and a point light to the world.
fn initialise_lights(world: &mut World)
{
    const AMBIENT_LIGHT_COLOUR: Rgba = Rgba(0.3, 0.3, 0.3, 1.0); // near-black
    const POINT_LIGHT_COLOUR: Rgba = Rgba(1.0, 1.0, 1.0, 1.0); // white
    const LIGHT_POSITION: [f32; 3] = [2.0, 2.0, -2.0];
    const LIGHT_RADIUS: f32 = 25.0;
    const LIGHT_INTENSITY: f32 = 3.0;

    // Add ambient light.
    world.add_resource(a_renderer::AmbientColor(AMBIENT_LIGHT_COLOUR));

    let light: a_renderer::Light = a_renderer::PointLight {
        center: LIGHT_POSITION.into(),
        radius: LIGHT_RADIUS,
        intensity: LIGHT_INTENSITY,
        color: POINT_LIGHT_COLOUR,
        ..Default::default()
        }.into();

    // Add point light.
    world.create_entity().with(light).build();
}


/// This function initialises a camera and adds it to the world.
fn initialise_camera(world: &mut World) {
    let transform = CameraMoveSystem::new().get_matrix();
    world
        .create_entity()
        .with(a_renderer::Camera::from(a_renderer::Projection::perspective(1.3, Deg(60.0))))
        .with(Transform(transform.into()))
        .build();
}

struct CameraMoveSystem
{
    angle_deg: f32,
}
impl CameraMoveSystem
{
    fn new() -> CameraMoveSystem
    {
        CameraMoveSystem {
            angle_deg: 180.,
            }
    }

    fn get_matrix(&self) -> amethyst::core::cgmath::Matrix4<f32>
    {
        use amethyst::core::cgmath::Matrix4;
        Matrix4::from_angle_x(Deg(90.))
            * Matrix4::from_angle_y(Deg(self.angle_deg))
            * Matrix4::from_translation([0.0, 5.0, 50.0].into())
    }
}
impl<'s> ecs::System<'s> for CameraMoveSystem
{
    type SystemData = (
        ecs::ReadStorage<'s, a_renderer::Camera>,
        ecs::WriteStorage<'s, Transform>,
        ecs::Fetch<'s, ::amethyst::input::InputHandler<String,String>>,
        );
    fn run(&mut self, (cam, mut transform, input): Self::SystemData)
    {
        let (_c, transform) = ecs::Join::join((&cam, &mut transform)).into_iter().next().unwrap();

        let mut update = false;
        for k in input.keys_that_are_down()
        {
            match k
            {
            ::amethyst::renderer::VirtualKeyCode::A => {
                self.angle_deg -= 1.;
                if self.angle_deg <= -180. {
                    self.angle_deg += 360.;
                }
                update = true;
                },
            ::amethyst::renderer::VirtualKeyCode::D => {
                self.angle_deg += 1.;
                if self.angle_deg <= -180. {
                    self.angle_deg -= 360.;
                }
                update = true;
                },
            _ => {},
            }
        }
        if update
        {
            transform.0 = self.get_matrix();
        }
    }
}