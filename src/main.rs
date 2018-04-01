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
use amethyst::core::cgmath::Matrix4;

use amethyst::renderer as a_renderer;

mod datafile;

type BoxError = Box<::std::error::Error>;

#[derive(Copy,Clone,Debug)]
#[allow(dead_code)]
enum PodName
{
    Startup,
    Game,
}
#[derive(Copy,Clone,Debug)]
#[allow(dead_code)]
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

struct EntityDef
{
    class: u8,
    model_a: String,
    model_b: String,

    drops: [(f32,u8); 2],

    description: String,
}
struct EntityRef
{
    ty: usize,
    flags: u16,
    // TODO: Use a 12.20 fixed point?
    // - That's my guess of the type in the input files here.
    x: f64,
    y: f64,
    z: f64,
}

fn main()
{
    env_logger::init();
    main_res().unwrap();
}
fn main_res() -> Result<(), BoxError>
{
    let pipe = ::amethyst::renderer::Pipeline::build().with_stage(
        ::amethyst::renderer::Stage::with_backbuffer()
            .clear_target(Rgba(0.,0.2,0.,0.), 1.0)
            //.with_pass(::amethyst::renderer::DrawShadedSeparate::new())
            .with_pass(::amethyst::renderer::DrawFlatSeparate::new())
            ,
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
            emission: loader.load_from_data([0.0, 0.0, 0.0, 1.0].into(), (), &tex_storage),
            ..mat_defaults.0.clone()
            }
    }

    fn load_model(&mut self, world: &mut World, model_path: DataPath) -> Result<(::amethyst::assets::Handle<a_renderer::Mesh>, a_renderer::Material), BoxError>
    {
        const SCALE: f32 = 1. / 100.;
        let m = datafile::Model::from_bin_file( self.pods.open_file(model_path)? )?;
        let vertices_as_arrays: Vec<_> = m.faces.iter()
            .flat_map(|v| v.v.iter().map(|&v| m.vertices[v as usize]))
            .map(|v| [v[0] * SCALE, v[1] * SCALE, v[2] * SCALE])
            .collect();
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
            -> Result< (a_renderer::Material, Vec<(usize, usize, usize)>), BoxError>
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
        // TODO: Pack the textures into an efficient format
        let mut sizes = Vec::new();
        for &name in &file_list
        {
            let size = self.pods.open_file( datapath!(Game, Art, name) )?.size();
            let dim = (size as f64).sqrt() as usize;
            assert_eq!(dim*dim, size);
            sizes.push(dim);
        }
        let max_width = sizes.iter().cloned().max().unwrap();
        //   - Check that all sizes are powers of two?
        // - Pack into a strip, with the width being the max texture width.
        let (subtex_coords, total_height) = {
            // - Sort by size (remembering index)
            let mut indexes: Vec<_> = (0.. sizes.len()).collect();
            indexes.sort_by_key(|&v| sizes[v]);
            let mut subtex_coords = vec![ (0,0,0); sizes.len() ];
            let mut h = 0;
            let mut x_space = 0;
            for v in indexes
            {
                if x_space < sizes[v] {
                    x_space = max_width;
                    h += sizes[v];
                }

                subtex_coords[v] = (max_width - x_space, h - sizes[v], sizes[v]);

                x_space -= sizes[v];
            }
            (subtex_coords, h)
            };
        debug!("load_level_texture: {} sources, total {}x{}", sizes.len(), max_width, total_height);
        
        // 2. Create a maxheight by total_width texture (RGBA)
        let pitch = max_width*4;
        let mut tex_data = vec![ 0; total_height*pitch ];

        // 3. Re-load every texture into the file.
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

            assert_eq!(subtex_coords[i].2, dim);
            let mut ofs = subtex_coords[i].1 * pitch + subtex_coords[i].0 * 4;
            debug!("load_level_texture: {} {:?} @ {},{}+{} - ofs={:#x} dim={}",
                i, name,
                subtex_coords[i].0, subtex_coords[i].1, subtex_coords[i].2,
                ofs, dim);
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
        }
        debug!("Loaded texture set {:?} - {}KiB RGBA uncompressed", list_file, tex_data.len() / 1024);

        if true
        {
            use std::io::Write;
            use byteorder::{WriteBytesExt, LittleEndian};
            let mut fh = ::std::fs::File::create("out.bmp")?;
            fh.write(&[b'B', b'M'])?;
            fh.write_u32::<LittleEndian>( (14+40+tex_data.len()) as u32 )?;   // Total size
            fh.write_u16::<LittleEndian>(0)?;   // resvd
            fh.write_u16::<LittleEndian>(0)?;   // resvd
            fh.write_u32::<LittleEndian>(14+40)?;   // Pixel data ofs
            fh.write_u32::<LittleEndian>(40)?;   // DIB header size
            fh.write_u32::<LittleEndian>((pitch / 4) as u32)?;  // W
            fh.write_u32::<LittleEndian>(total_height as u32)?; // H
            fh.write_u16::<LittleEndian>(1)?;   // nplanes
            fh.write_u16::<LittleEndian>(32)?;  // bpp
            fh.write_u32::<LittleEndian>(0)?;   // compression
            fh.write_u32::<LittleEndian>(0)?;   // size (can be zero)
            fh.write_u32::<LittleEndian>(10000)?;   // hres
            fh.write_u32::<LittleEndian>(10000)?;   // vres
            fh.write_u32::<LittleEndian>(0)?;   // nimportant
            fh.write_u32::<LittleEndian>(0)?;   // nimportant
            fh.write(&tex_data)?;
        }

        let loader = world.read_resource::<::amethyst::assets::Loader>();
        let tex = a_renderer::TextureData::U8(tex_data,
            a_renderer::TextureMetadata {
                sampler: Some(::gfx_core::texture::SamplerInfo::new(
                    ::gfx_core::texture::FilterMethod::Bilinear,
                    ::gfx_core::texture::WrapMode::Clamp,
                    )),  // TODO: Add a sampler?
                mip_levels: None,
                size: Some(( (pitch/4) as u16, total_height as u16 )),
                dynamic: false,
                format: Some(::gfx_core::format::SurfaceType::R8_G8_B8_A8),
                channel: None,//Some(::gfx_core::format::ChannelType::Uint),
                }
            );
        let tex: a_renderer::TextureHandle = loader.load_from_data(tex, (), &world.read_resource());

        let mat_defaults = world.read_resource::<a_renderer::MaterialDefaults>();

        Ok( (a_renderer::Material {
            albedo: tex,
            emission: loader.load_from_data([0.0, 0.0, 0.0, 1.0].into(), (), &world.read_resource()),
            //albedo: {
            //    let albedo = [0.5, 0.5, 0.5, 1.0].into();
            //    loader.load_from_data(albedo, (), &world.read_resource())
            //    },
            ..mat_defaults.0.clone()
            }, subtex_coords) )
    }

    fn load_heightmap(&mut self, world: &mut World, model_path: DataPath, texture_widths: &[(usize,usize,usize)]) -> Result<::amethyst::assets::Handle<a_renderer::Mesh>, BoxError>
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
            let h_scale = 1. / 256.;
            let xy_scale = 1. / 8.;

            let mut triangle_verts = vec![];
            with_row_pairs(&mut self.pods, model_path, |r, prev_row, cur_row| {
                assert_eq!( prev_row.len(), cur_row.len() );
                let xy_ofs = (cur_row.len() / 2) as f32 * xy_scale;
                // Make triangles for each quad.
                for c in 1 .. prev_row.len()
                {
                    let pt_tl = [(c-1) as f32 * xy_scale - xy_ofs, prev_row[c-1] as f32 * h_scale, (r-1) as f32 * xy_scale - xy_ofs, ];
                    let pt_tr = [(c  ) as f32 * xy_scale - xy_ofs, prev_row[c  ] as f32 * h_scale, (r-1) as f32 * xy_scale - xy_ofs, ];
                    let pt_bl = [(c-1) as f32 * xy_scale - xy_ofs, cur_row [c-1] as f32 * h_scale, (r  ) as f32 * xy_scale - xy_ofs, ];
                    let pt_br = [(c  ) as f32 * xy_scale - xy_ofs, cur_row [c  ] as f32 * h_scale, (r  ) as f32 * xy_scale - xy_ofs, ];
                    // BottomLeft, TopRight, TopLeft
                    triangle_verts.push(pt_bl);
                    triangle_verts.push(pt_tr);
                    triangle_verts.push(pt_tl);
                    // BottomLeft, BottomRight, TopRight
                    triangle_verts.push( pt_bl );
                    triangle_verts.push( pt_br );
                    triangle_verts.push( pt_tr );
                }

                })?;
            triangle_verts
            };
        
        // TODO: Textures
        // - Load a massive texture from the flies listed in `LEVEL.TEX`
        // - Get indexes from `LEVEL.CLR`
        let tex_coords: Vec<a_renderer::Separate<a_renderer::TexCoord>> = {
            let tex_width : usize = texture_widths.iter().map(|v| v.0+v.2).max().unwrap();
            let tex_height: usize = texture_widths.iter().map(|v| v.1+v.2).max().unwrap();

            let mut tex_coords = Vec::with_capacity(triangle_verts.len());
            with_row_pairs(&mut self.pods, datapath!(Game, Data, &clr_fname), |_r, prev_row, _cur_row| {
                for c in 1 .. prev_row.len()
                {
                    let tex_id = prev_row[c-1] as usize;
                    let base_u = texture_widths[tex_id].0 as f32 / tex_width as f32;
                    let base_v = texture_widths[tex_id].1 as f32 / tex_height as f32;
                    let w = (texture_widths[tex_id].2 as f32 - 1.) / tex_width as f32;
                    let h = (texture_widths[tex_id].2 as f32 - 1.) / tex_height as f32;

                    let tex_tl = [ base_u  , 1.0 - (base_v  ) ];    // Lowest  U, Lowest V
                    let tex_tr = [ base_u+w, 1.0 - (base_v  ) ];    // Highest U, Lowest V
                    let tex_bl = [ base_u  , 1.0 - (base_v+h) ];    // Lowest  U, Highest V
                    let tex_br = [ base_u+w, 1.0 - (base_v+h) ];    // Highest U, Highest V
                    //let (tex_tl, tex_tr, tex_br, tex_bl, ) = (tex_tr, tex_bl, tex_tl, tex_br, );
                    // - TODO: Fix texturing, assignment is wrong again.
                    // BottomLeft, TopRight, TopLeft
                    tex_coords.push( a_renderer::Separate::new(tex_bl) );
                    tex_coords.push( a_renderer::Separate::new(tex_tr) );
                    tex_coords.push( a_renderer::Separate::new(tex_tl) );
                    // BottomLeft, BottomRight, TopRight
                    tex_coords.push( a_renderer::Separate::new(tex_bl) );
                    tex_coords.push( a_renderer::Separate::new(tex_br) );
                    tex_coords.push( a_renderer::Separate::new(tex_tr) );
                }
                })?;
            tex_coords
            };
        let normals = triangle_verts.iter()
            .map(|_v| {
                // TODO: Calculate a normal based on the surface.
                a_renderer::Separate::new([ 0.0, 1.0, 0.0 ])
                })
            .collect()
            ;

        let m2: a_renderer::ComboMeshCreator = (
            triangle_verts.into_iter().map(|p| a_renderer::Separate::<a_renderer::Position>::new(p)).collect::<Vec<_>>(),
            None,   // Colours
            Some(tex_coords),   // Texture coords (needed)
            Some(normals),   // Normals
            None,   // TODO: Tangents?
            ).into();

        let loader = world.read_resource::<::amethyst::assets::Loader>();
        let mesh: ::amethyst::assets::Handle<a_renderer::Mesh> = loader.load_from_data(m2.into(), (), &world.read_resource());

        Ok( mesh )
    }

    fn load_entities_file(&mut self, path: DataPath) -> Result<(Vec<EntityDef>, Vec<EntityRef>), BoxError>
    {
        let mut fp = ::std::io::BufReader::new( self.pods.open_file(path)? );

        fn read_line<'a, F: ::std::io::BufRead>(f: &mut F, line: &'a mut String) -> Result<&'a str, BoxError> {
            line.clear();
            f.read_line(line)?;
            let len = line.trim_right().len();
            line.truncate(len);
            Ok(&line[..])
        }
        let mut line = String::new();

        // 1. Read the entity count.
        let ty_count: usize = read_line(&mut fp, &mut line)?.parse().unwrap();
        let mut def_list = Vec::new();
        for _ in 0 .. ty_count
        {
            let class: u8;
            let model: String;
            let model_destroyed: String;
            let drops: [(u8,i8); 2];
            let desc;
            // Line 1: Model/general information
            {
                read_line(&mut fp, &mut line)?;
                let mut it = line.split(',');
                class = it.next().unwrap().parse().unwrap();
                let _ = it.next().unwrap();
                let _ = it.next().unwrap();
                let _ = it.next().unwrap();
                let _ = it.next().unwrap();
                let _ = it.next().unwrap();
                model = it.next().unwrap().to_owned();
                model_destroyed = it.next().unwrap().to_owned();
            }
            // Line 2: Unknown
            {
                read_line(&mut fp, &mut line)?;
                let mut it = line.split(',');
                let _ = it.next().unwrap();
                let _ = it.next().unwrap();
                let _ = it.next().unwrap();
                let _ = it.next().unwrap();
                let _ = it.next().unwrap();
            }
            // Line 3: Drop info
            {
                read_line(&mut fp, &mut line)?;
                let mut it = line.split(',');
                drops = [
                    (it.next().unwrap().parse().expect("drop perc 1"), it.next().unwrap().parse().expect("drop item 1"), ),
                    (it.next().unwrap().parse().expect("drop perc 2"), {let v = it.next().unwrap(); debug!("{:?}", v); v.parse().expect("drop item 2")}, ),
                    ];
            }
            // Line 4: Unknown
            {
                read_line(&mut fp, &mut line)?;
                let mut _it = line.split(',');
            }
            // Line 5: ";NewHit"
            {
                read_line(&mut fp, &mut line)?;
                assert_eq!(line, ";NewHit")
            }
            // Line 6: Unknown
            {
                read_line(&mut fp, &mut line)?;
                let mut _it = line.split(',');
            }
            // Line 7: "!NewAtakRet"
            {
                read_line(&mut fp, &mut line)?;
                assert_eq!(line, "!NewAtakRet")
            }
            // Line 8: Unknown
            {
                read_line(&mut fp, &mut line)?;
                let mut _it = line.split(',');
            }
            // Line 9: Description
            {
                read_line(&mut fp, &mut line)?;
                desc = line.clone();
                debug!("Entity description '{}'", desc);
            }
            // Line 10: "#New2ndweapon"
            {
                read_line(&mut fp, &mut line)?;
                assert_eq!(line, "#New2ndweapon")
            }
            // Line 11: Unknown
            {
                read_line(&mut fp, &mut line)?;
                let mut _it = line.split(',');
            }
            // Line 12: "%SFX"
            {
                read_line(&mut fp, &mut line)?;
                assert_eq!(line, "%SFX")
            }
            // Line 13: Unknown filename
            {
                read_line(&mut fp, &mut line)?;
            }
            // Line 14: Unknown filename
            {
                read_line(&mut fp, &mut line)?;
            }

            def_list.push(EntityDef {
                class: class,
                model_a: model,
                model_b: model_destroyed,

                drops: [
                    (drops[0].0 as f32 / 100., drops[0].1 as u8),
                    (drops[1].0 as f32 / 100., drops[1].1 as u8),
                    ],

                description: desc,
                });
        }

        // --------------
        let ent_count: usize = read_line(&mut fp, &mut line)?.parse().unwrap();
        let mut ent_list = Vec::new();
        for _ in 0 .. ent_count
        {
            read_line(&mut fp, &mut line)?;
            let mut it = line.split(',');
            
            let ty     : usize = it.next().expect("ent type").parse().expect("ent type");
            let flags  : u16   = it.next().expect("ent flags").parse().expect("ent flags");
            let x_fixed: i32   = it.next().expect("ent x").parse().expect("ent x");
            let y_fixed: i32   = it.next().expect("ent y").parse().expect("ent y");
            let z_fixed: i32   = it.next().expect("ent z").parse().expect("ent z");
            let _unk1  : u32   = it.next().expect("ent unk1").parse().expect("ent unk1");
            let _unk2  : u32   = it.next().expect("ent unk2").parse().expect("ent unk2");
            let _unk3  : u32   = it.next().expect("ent unk3").parse().expect("ent unk3");

            const COORD_SCALE_XZ: f64 = 1. / (1 << 20) as f64;
            const COORD_SCALE_Y: f64 = 1. / (1 << 20) as f64;
            ent_list.push(EntityRef {
                ty: ty,
                flags: flags,
                // TODO: Use a 12.20 fixed point?
                // - Note: X and Z have been switched, needed for the terrain to match
                x: x_fixed as f64 * COORD_SCALE_XZ / 8. + 32. - 16.,
                y: y_fixed as f64 * COORD_SCALE_Y  / 8.,
                z: z_fixed as f64 * COORD_SCALE_XZ / 8. + 32. - 16.,
                });
        }

        Ok( (def_list, ent_list) )
    }
}

impl State for GameRoot
{
    fn on_start(&mut self, world: &mut World)
    {
        // Load a random model (untextured)
        // DISABLED.
        if true
        {
            //let model_path = datapath!(Game, Models, "LEAFSHIP.BIN");
            let model_path = datapath!(Game, Models, "TENT2.BIN");
            let (mesh, material) = self.load_model(world, model_path).unwrap();
            world.create_entity()
                .with(Transform::default())
                .with(mesh)
                .with(material)
                .build()
                ;
        }

        // Load the "EGYPT" level from heightmap with its texture set
        if true
        {
            let (mat, tex_scales) = self.load_level_material(world, datapath!(Game, Data, "EGYPT.TEX"), datapath!(Game, Art, "EGYPT.ACT")).expect("Loading level tex");
            let mesh = self.load_heightmap(world, datapath!(Game, Data, "EGYPT.RAW"), &tex_scales).expect("Loading level");
            world.create_entity()
                .with(Transform::default())
                .with(mesh)
                .with(mat)
                .build()
                ;
        }

        // Load entities from the level entity file
        if true
        {
            let (entity_types, entity_list) = self.load_entities_file(datapath!(Game, Data, "EGYPT.DEF")).expect("Loading level entities");

            // - Load models for all entity types (and metadata?)
            let mut model_mats = Vec::new();
            for e in &entity_types
            {
                debug!("Load {:?} '{}'", e.model_a, e.description);
                let model_path = datapath!(Game, Models, &e.model_a);
                model_mats.push( self.load_model(world, model_path).unwrap() );
            }
            // - Place instances of those models into the world.
            for e in &entity_list//[..10]
            {
                debug!("@{:7.3},{:7.3},{:9.3} #{}", e.x, e.y, e.z, e.ty);


                //let model_path = datapath!(Game, Models, &entity_types[e.ty].model_a);
                //let (mesh, mat) = self.load_model(world, model_path).unwrap();
                let (mesh, mat) = model_mats[e.ty].clone();

                world.create_entity()
                    //.with(Transform::new())
                    .with(Transform(Matrix4::from_translation([e.x as f32, e.y as f32, e.z as f32].into())))
                    .with(mesh)
                    .with(mat)
                    .build()
                    ;
            }
            ::std::mem::forget(model_mats);
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
    const AMBIENT_LIGHT_COLOUR: Rgba = Rgba(0.3, 0.3, 0.3, 1.0); // near-black
    const POINT_LIGHT_COLOUR: Rgba = Rgba(1.0, 1.0, 1.0, 1.0); // white
    const LIGHT_POSITION: [f32; 3] = [2.0, 12.0, -2.0];
    const LIGHT_RADIUS: f32 = 128.0;
    const LIGHT_INTENSITY: f32 = 50.0;

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
    x: f32,
    y: f32,
    z: f32,
    tilt_deg: f32,
    angle_deg: f32,
}
impl CameraMoveSystem
{
    fn new() -> CameraMoveSystem
    {
        CameraMoveSystem {
            x: 0.,
            y: 2.0,
            z: 16.,
            tilt_deg: 0.,
            angle_deg: 0.,
            }
    }

    fn get_matrix(&self) -> amethyst::core::cgmath::Matrix4<f32>
    {
        Matrix4::from_scale(1.)
            * Matrix4::from_translation([self.x, self.y, self.z].into())
            * Matrix4::from_angle_y(Deg(self.angle_deg))
            * Matrix4::from_angle_x(Deg(self.tilt_deg))
    }

    fn shift(&mut self, angle: f32, step: f32)
    {
        self.x += angle.to_radians().sin() * step;
        self.z += angle.to_radians().cos() * step;
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
            const SPEED: f32 = 0.025;
            const VSPEED: f32 = 0.01;
            match k
            {
            ::amethyst::renderer::VirtualKeyCode::Left => {
                self.angle_deg += 1.;
                if self.angle_deg <= -180. {
                    self.angle_deg += 360.;
                }
                update = true;
                },
            ::amethyst::renderer::VirtualKeyCode::Right => {
                self.angle_deg -= 1.;
                if self.angle_deg <= -180. {
                    self.angle_deg -= 360.;
                }
                update = true;
                },
            ::amethyst::renderer::VirtualKeyCode::Down => {
                self.tilt_deg -= 1.;
                if self.tilt_deg <= -90. {
                    self.tilt_deg = -90.;
                }
                update = true;
                },
            ::amethyst::renderer::VirtualKeyCode::Up => {
                self.tilt_deg += 1.;
                if self.tilt_deg >= 90. {
                    self.tilt_deg = 90.;
                }
                update = true;
                },
            ::amethyst::renderer::VirtualKeyCode::W => {
                let a = self.angle_deg;
                self.shift(a, -SPEED);
                update = true;
                },
            ::amethyst::renderer::VirtualKeyCode::S => {
                let a = self.angle_deg - 180.;
                self.shift(a, -SPEED);
                update = true;
                },
            ::amethyst::renderer::VirtualKeyCode::A => {
                let a = self.angle_deg - 90.;
                self.shift(a, SPEED);
                update = true;
                },
            ::amethyst::renderer::VirtualKeyCode::D => {
                let a = self.angle_deg + 90.;
                self.shift(a, SPEED);
                update = true;
                },
            ::amethyst::renderer::VirtualKeyCode::R => {
                self.y += VSPEED;
                update = true;
                },
            ::amethyst::renderer::VirtualKeyCode::F => {
                self.y -= VSPEED;
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