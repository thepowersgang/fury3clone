pub struct Model
{
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<Face>,
}
pub struct Face
{
    pub v: [usize; 3],
    pub normal: [f32; 3],
}

impl Model
{
    pub fn from_bin_file<F: ::std::io::Read>(mut file: F) -> ::std::io::Result<Model>
    {
        use byteorder::ReadBytesExt;
        use byteorder::LittleEndian;

        let id = file.read_u32::<LittleEndian>()?;
        if id != 0x14 {
            return Err(::std::io::Error::new(::std::io::ErrorKind::InvalidData, "File ID not 0x14"))
        }

        let scale = file.read_u32::<LittleEndian>()?;
        let _unk1 = file.read_u32::<LittleEndian>()?;
        let _unk2 = file.read_u32::<LittleEndian>()?;
        let num_vert = file.read_u32::<LittleEndian>()?;

        let mut vertices = Vec::new();
        for _ in 0 .. num_vert
        {
            let x = scale as f32 / file.read_i32::<LittleEndian>()? as f32;
            let y = scale as f32 / file.read_i32::<LittleEndian>()? as f32;
            let z = scale as f32 / file.read_i32::<LittleEndian>()? as f32;
            vertices.push([x, y, z]);
        }

        let mut faces = Vec::new();
        loop
        {
            let block_id = match file.read_u32::<LittleEndian>()
                {
                Ok(v) => v,
                Err(ref e) if e.kind() == ::std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
                };
            match block_id
            {
            0x00 => {
                debug!("0x00: EOF");
                },
            // Texture Block (sets the current texture)
            0x0D => {
                let _unk1 = file.read_u32::<LittleEndian>()?;
                let texture_name = super::CStrBuf::read_from_file(&mut file, [0u8; 16])?;
                debug!("0x0D: texture_name={:?}", &*texture_name);
                },
            // 0x0E => Faces
            0x0E | 0x18 => {
                let nvert = file.read_u32::<LittleEndian>()?;
                let normal_x = file.read_i32::<LittleEndian>()?;
                let normal_y = file.read_i32::<LittleEndian>()?;
                let normal_z = file.read_i32::<LittleEndian>()?;
                let _magic = file.read_u32::<LittleEndian>()?;
                //debug!("0x00: EOF");

                let normal = [
                    normal_x as f32 / 65535.0,
                    normal_y as f32 / 65535.0,
                    normal_z as f32 / 65535.0,
                    ];
                
                if nvert == 3
                {
                    let mut face_indexes = [0,0,0];
                    for slot in face_indexes.iter_mut()
                    {
                        let idx = file.read_u32::<LittleEndian>()?;
                        let _tex_u = file.read_u32::<LittleEndian>()?;
                        let _tex_v = file.read_u32::<LittleEndian>()?;
                        *slot = idx as usize;
                    }
                    faces.push(Face {
                        v: face_indexes,
                        normal: normal,
                        });
                }
                else if nvert == 4
                {
                    let mut fi = [0,0,0,0];
                    for slot in fi.iter_mut()
                    {
                        let idx = file.read_u32::<LittleEndian>()?;
                        let _tex_u = file.read_u32::<LittleEndian>()?;
                        let _tex_v = file.read_u32::<LittleEndian>()?;
                        *slot = idx as usize;
                    }

                    faces.push(Face {
                        v: [fi[0], fi[1], fi[2]],
                        normal: normal,
                        });
                    faces.push(Face {
                        v: [fi[0], fi[3], fi[2]],
                        normal: normal,
                        });
                }
                else
                {
                    panic!("TODO: Strange number of points in face - {}", nvert);
                }
                },
            // 0x17 : Unknown purpose
            0x17 => {
                let _unk1 = file.read_u32::<LittleEndian>()?;
                let _unk2 = file.read_u32::<LittleEndian>()?;
                //debug!("")
                },
            _ => panic!("TODO: MTM block 0x{:02x}", block_id),
            }
        }

        Ok(Model {
            vertices: vertices,
            faces: faces,
            })
    }
}