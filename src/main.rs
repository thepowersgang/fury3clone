extern crate amethyst;
extern crate byteorder;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate gfx_core;

use amethyst::prelude::*;
use amethyst::renderer::Rgba;
use amethyst::renderer::Event;
use amethyst::core::transform::Transform;
use amethyst::core::cgmath::Deg;
use amethyst::core::cgmath::Vector3;
use amethyst::ecs;

use amethyst::renderer as a_renderer;

mod datafile;

#[derive(Copy,Clone,Debug)]
enum PodName
{
    Startup,
    Game,
}
#[derive(Copy,Clone,Debug)]
enum DataFolder
{
    Art,
    Data,
    Demo,
    Fog,
    Levels,
    Models,
    Music,
    Sound,
    Startup,
}
#[derive(Copy,Clone,Debug)]
struct DataPath<'a>
{
    archive: PodName,
    folder: DataFolder,
    file: &'a str,
}
macro_rules! datapath {
    ($a:ident, $d:ident, $f:expr) => ( DataPath { archive: PodName::$a, folder: DataFolder::$d, file: $f, } );
}


struct GameRoot
{
    pods: PodFiles,
}
struct PodFiles
{
    startup: self::datafile::PodArchive,
    game: self::datafile::PodArchive,
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
        pods: PodFiles {
            startup: self::datafile::PodArchive::from_file(r"V:\Games\Fury3\SYSTEM\STARTUP.POD")?,
            game: self::datafile::PodArchive::from_file(r"V:\Games\Fury3\SYSTEM\FURY3.POD")?,
            }
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

impl PodFiles
{
    fn open_file(&mut self, path: DataPath) -> Result<datafile::FileHandle, ::std::io::Error>
    {
        let a = match path.archive
            {
            PodName::Startup => &mut self.startup,
            PodName::Game => &mut self.game,
            };
        let dirname = match path.folder
            {
            DataFolder::Art => "ART",
            DataFolder::Data => "DATA",
            DataFolder::Demo => "DEMO",
            DataFolder::Fog => "FOG",
            DataFolder::Levels => "LEVELS",
            DataFolder::Models => "MODELS",
            DataFolder::Music => "MUSIC",
            DataFolder::Sound => "SOUND",
            DataFolder::Startup => "STARTUP",
            };
        a.open_dir_file(dirname, path.file)
    }
}

impl GameRoot
{
    fn load_blue_material(&mut self, world: &mut World) -> a_renderer::Material
    {
        // Colour/material
        let tex_storage = world.read_resource();
        let mat_defaults = world.read_resource::<a_renderer::MaterialDefaults>();

        let loader = world.read_resource::<::amethyst::assets::Loader>();
        let albedo = [0.0, 0.0, 1.0, 1.0].into();
        let albedo = loader.load_from_data(albedo, (), &tex_storage);
        a_renderer::Material {
            albedo,
            ..mat_defaults.0.clone()
            }
    }

    fn load_model(&mut self, world: &mut World, model_path: DataPath) -> Result<(::amethyst::assets::Handle<a_renderer::Mesh>, a_renderer::Material), Box<::std::error::Error>>
    {
        let m = datafile::Model::from_bin_file( self.pods.open_file(model_path)? )?;
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

        let mesh: ::amethyst::assets::Handle<a_renderer::Mesh> = {
            let loader = world.read_resource::<::amethyst::assets::Loader>();
            let m2: a_renderer::ComboMeshCreator = (
                vertices_as_arrays.into_iter().map(|p| a_renderer::Separate::<a_renderer::Position>::new(p)).collect::<Vec<_>>(),
                None,   // TODO: Colours
                Some(tex_coords),   // Texture coords (needed)
                Some(normals),   // TODO: Normals
                None,   // TODO: Tangents
                ).into();
            loader.load_from_data(m2.into(), (), &world.read_resource())
            };

        let mat = self.load_blue_material(world);

        Ok( (mesh, mat) )
    }

    fn load_level_material(&mut self, world: &mut World, list_file: DataPath, default_plt: DataPath)
            -> Result< (a_renderer::Material, Vec<usize>), Box<::std::error::Error>>
    {   
        use ::std::io::Read;

        let mut file_list_data = String::new();
        let file_list: Vec<&str> = {
            let mut list_file = self.pods.open_file(list_file)?;
            list_file.read_to_string(&mut file_list_data)?;
            let mut it = file_list_data.split("\r\n");
            let _count = it.next();
            let mut v: Vec<_> = it.collect();
            v.pop();
            v
            };

        let default_plt = {
            let mut fh = self.pods.open_file( default_plt )?;
            let mut rv = vec![0; 256*3];
            fh.read(&mut rv)?;
            rv
            };

        // 1. Determine max texture size
        let mut total_width = 0;
        let mut max_height = 0;
        let mut sizes = Vec::new();
        for &name in &file_list
        {
            let size = self.pods.open_file( datapath!(Game, Art, name) )?.size();
            let dim = (size as f64).sqrt() as usize;
            assert_eq!(dim*dim, size);
            sizes.push(dim);
            total_width += dim;
            max_height = ::std::cmp::max(max_height, dim);
        }
        // 2. Create a maxheight by total_width texture (RGBA)
        let pitch = total_width*4;
        let mut tex_data = vec![ 0; max_height*pitch ];

        // 3. Re-load every texture into the file.
        let mut start_ofs = 0;
        for (i, &name) in file_list.iter().enumerate()
        {
            // Make a second version of the name that is .ACT instead of .RAW
            let self_plt;
            let palette = {
                    let act_fname = format!("{}ACT", &name[..name.len() - 3]);
                    if let Ok(mut fh) = self.pods.open_file( datapath!(Game, Art, &act_fname) ) {
                        let mut rv = vec![0; 256*3];
                        fh.read(&mut rv)?;
                        self_plt = rv;
                        &self_plt
                    }
                    else {
                        &default_plt
                    }
                };
            let mut fh = self.pods.open_file( datapath!(Game, Art, name) )?;
            let dim = (fh.size() as f64).sqrt() as usize;

            let mut ofs = start_ofs;
            debug!("load_level_texture: {} {:?} - ofs={:#x} dim={}", i, name, ofs, dim);

            for _ in 0 .. dim
            {
                //debug!("> ofs={:#x}", ofs);
                let dst = &mut tex_data[ofs ..][.. dim*4];

                for x in 0 .. dim
                {
                    let b = {
                        let mut b = [0];
                        fh.read(&mut b)?;
                        b[0]
                        };
                    dst[x*4 + 0] = palette[b as usize * 3 + 0];
                    dst[x*4 + 1] = palette[b as usize * 3 + 1];
                    dst[x*4 + 2] = palette[b as usize * 3 + 2];
                    dst[x*4 + 3] = 255;
                }

                ofs += pitch;
            }
            debug!("> ofs={:#x} / {:#x}", ofs, tex_data.len());
            start_ofs += dim * 4;
        }
        debug!("Loaded texture set {:?} - {}KiB RGBA uncompressed", list_file, tex_data.len() / 1024);

        let loader = world.read_resource::<::amethyst::assets::Loader>();
        let tex = a_renderer::TextureData::U8(tex_data,
            a_renderer::TextureMetadata {
                sampler: None,
                mip_levels: None,
                size: Some(( (pitch/4) as u16, max_height as u16 )),
                dynamic: false,
                format: Some(::gfx_core::format::SurfaceType::R8_G8_B8_A8),
                channel: Some(::gfx_core::format::ChannelType::Uint),
                }
            );
        let tex = loader.load_from_data(tex, (), &world.read_resource());

        let mat_defaults = world.read_resource::<a_renderer::MaterialDefaults>();

        Ok( (a_renderer::Material {
            albedo: tex,
            ..mat_defaults.0.clone()
            }, sizes) )
    }

    fn load_heightmap(&mut self, world: &mut World, model_path: DataPath, texture_widths: &[usize]) -> Result<::amethyst::assets::Handle<a_renderer::Mesh>, Box<::std::error::Error>>
    {
        use std::io::Read;
        let clr_fname = format!("{}CLR", &model_path.file[..model_path.file.len() - 3]);

        fn with_row_pairs<F>(pods: &mut PodFiles, p: DataPath, mut cb: F) -> ::std::io::Result<()>
        where
            F: FnMut(usize, &[u8], &[u8])
        {
            let mut file = pods.open_file(p)?;
            let dim = (file.size() as f64).sqrt() as usize;
            assert_eq!(dim*dim, file.size());

            let mut prev_row = vec![0u8; dim];
            let mut cur_row = vec![0u8; dim];
            file.read(&mut cur_row)?;
            for r in 1 .. dim
            {
                ::std::mem::swap(&mut prev_row, &mut cur_row);
                file.read(&mut cur_row)?;

                cb(r, &prev_row, &cur_row);
            }
            Ok( () )
        }

        let triangle_verts = {
            let h_scale = 8. / 256.;
            let xy_scale = 1. / 8.;

            let mut triangle_verts = vec![];
            with_row_pairs(&mut self.pods, model_path, |r, prev_row, cur_row| {
                assert_eq!( prev_row.len(), cur_row.len() );
                // Make triangles for each quad.
                for c in 1 .. prev_row.len()
                {
                    // TopLeft, TopRight, BottomLeft
                    triangle_verts.push( [(r-1) as f32 * xy_scale, prev_row[c-1] as f32 * h_scale, (c-1) as f32 * xy_scale] );
                    triangle_verts.push( [(r-1) as f32 * xy_scale, prev_row[c  ] as f32 * h_scale, (c  ) as f32 * xy_scale] );
                    triangle_verts.push( [(r  ) as f32 * xy_scale, cur_row [c-1] as f32 * h_scale, (c-1) as f32 * xy_scale] );
                    
                    // TopRight, BottomRight, BottomLeft
                    triangle_verts.push( [(r-1) as f32 * xy_scale, prev_row[c  ] as f32 * h_scale, (c  ) as f32 * xy_scale] );
                    triangle_verts.push( [(r  ) as f32 * xy_scale, cur_row [c  ] as f32 * h_scale, (c  ) as f32 * xy_scale] );
                    triangle_verts.push( [(r  ) as f32 * xy_scale, cur_row [c-1] as f32 * h_scale, (c-1) as f32 * xy_scale] );
                }

                });
            triangle_verts
            };
        
        // TODO: Textures
        // - Load a massive texture from the flies listed in `LEVEL.TEX`
        // - Get indexes from `LEVEL.CLR`
        let tex_coords: Vec<a_renderer::Separate<a_renderer::TexCoord>> = {
            let texwidth: usize = texture_widths.iter().cloned().sum();
            let mut tex_coords = Vec::with_capacity(triangle_verts.len());
            with_row_pairs(&mut self.pods, datapath!(Game, Data, &clr_fname), |_r, prev_row, _cur_row| {
                for c in 1 .. prev_row.len()
                {
                    let tex_id = prev_row[c-1] as usize;
                    let ofs: usize = texture_widths[..tex_id].iter().cloned().sum();
                    let base = ofs as f32 / texwidth as f32;
                    let w = texture_widths[tex_id] as f32 / texwidth as f32;
                    let h = 1.0;    // TODO.
                    // TopLeft, TopRight, BottomLeft
                    tex_coords.push( a_renderer::Separate::new([base  , 0.0]) );
                    tex_coords.push( a_renderer::Separate::new([base+w, 0.0]) );
                    tex_coords.push( a_renderer::Separate::new([base  , h  ]) );

                    // TopRight, BottomRight, BottomLeft
                    tex_coords.push( a_renderer::Separate::new([base+w, 0.0]) );
                    tex_coords.push( a_renderer::Separate::new([base+w, h  ]) );
                    tex_coords.push( a_renderer::Separate::new([base  , h  ]) );
                }
                });
            tex_coords
            };
        let normals = triangle_verts.iter()
            .map(|_v| {
                a_renderer::Separate::new([ 0.0, 1.0, 0.0 ])
                })
            .collect()
            ;

        let m2: a_renderer::ComboMeshCreator = (
            triangle_verts.into_iter().map(|p| a_renderer::Separate::<a_renderer::Position>::new(p)).collect::<Vec<_>>(),
            None,   // Colours
            Some(tex_coords),   // Texture coords (needed)
            Some(normals),   // TODO: Normals
            None,   // TODO: Tangents
            ).into();

        let loader = world.read_resource::<::amethyst::assets::Loader>();
        let mesh: ::amethyst::assets::Handle<a_renderer::Mesh> = loader.load_from_data(m2.into(), (), &world.read_resource());

        Ok( mesh )
    }
}

impl State for GameRoot
{
    fn on_start(&mut self, world: &mut World)
    {

        if false
        {
            // Add the first level as a model.
            //let model_path = datapath!(Game, Models, "LEAFSHIP.BIN");
            let model_path = datapath!(Game, Models, "EGYPT.BIN");
            //let model_path = "DATA\\EGYPT.BIN";
            let (mesh, material) = self.load_model(world, model_path).unwrap();
            world.create_entity()
                .with(Transform::default())
                .with(mesh)
                .with(material)
                .build()
                ;
        }

        if true
        {
            let (mat, tex_scales) = self.load_level_material(world, datapath!(Game, Data, "EGYPT.TEX"), datapath!(Game, Art, "EGYPT.ACT")).expect("Loading level tex");
            //let mat = self.load_blue_material(world);
            let mesh = self.load_heightmap(world, datapath!(Game, Data, "EGYPT.RAW"), &tex_scales).expect("Loading level");
            world.create_entity()
                .with(Transform::default())
                .with(mesh)
                .with(mat)
                .build()
                ;
        }

            
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
                _ => Trans::None,
            },
            _ => Trans::None,
        }
    }
}


/// This function adds an ambient light and a point light to the world.
fn initialise_lights(world: &mut World)
{
    const AMBIENT_LIGHT_COLOUR: Rgba = Rgba(0.9, 0.3, 0.3, 1.0); // near-black
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
    z: f32,
    tilt_deg: f32,
    angle_deg: f32,
}
impl CameraMoveSystem
{
    fn new() -> CameraMoveSystem
    {
        CameraMoveSystem {
            z: 50.,
            tilt_deg: 0.,
            angle_deg: 180.,
            }
    }

    fn get_matrix(&self) -> amethyst::core::cgmath::Matrix4<f32>
    {
        use amethyst::core::cgmath::Matrix4;
        Matrix4::from_scale(1.)
            * Matrix4::from_angle_x(Deg(self.tilt_deg))
            * Matrix4::from_angle_y(Deg(self.angle_deg))
            * Matrix4::from_translation([0.0, 5.0, self.z].into())
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
            ::amethyst::renderer::VirtualKeyCode::Left => {
                self.angle_deg -= 1.;
                if self.angle_deg <= -180. {
                    self.angle_deg += 360.;
                }
                update = true;
                },
            ::amethyst::renderer::VirtualKeyCode::Right => {
                self.angle_deg += 1.;
                if self.angle_deg <= -180. {
                    self.angle_deg -= 360.;
                }
                update = true;
                },
            ::amethyst::renderer::VirtualKeyCode::Down => {
                self.tilt_deg -= 1.;
                if self.tilt_deg <= -90. {
                    self.tilt_deg += 180.;
                }
                update = true;
                },
            ::amethyst::renderer::VirtualKeyCode::Up => {
                self.tilt_deg += 1.;
                if self.tilt_deg <= -90. {
                    self.tilt_deg -= 180.;
                }
                update = true;
                },
            ::amethyst::renderer::VirtualKeyCode::W => {
                self.z -= 1.0;
                update = true;
                },
            ::amethyst::renderer::VirtualKeyCode::S => {
                self.z += 1.0;
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