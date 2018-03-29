pub struct PodArchive
{
    file: ::std::fs::File,
    /// Sorted list of files
    files: Vec<FileEnt>,
}

struct FileEnt
{
    name: super::CStrBuf<[u8; 32]>,
    offset: u32,
    size: u32,
}

impl PodArchive
{
    pub fn from_file<P: AsRef<::std::path::Path>>(path: P) -> ::std::io::Result<PodArchive>
    {

        use std::io::Read;
        use byteorder::ReadBytesExt;
        use byteorder::LittleEndian;
        let mut fp = ::std::fs::File::open(path.as_ref())?;

        // - Read header
        let file_count = fp.read_u32::<LittleEndian>()?;
        let _comment = {
            let mut buf = [0; 0x50];
            fp.read(&mut buf)?;
            //Ok( super::CStrBuf::new(&buf) )
            buf
            };
        debug!("Loading {} files from {}", file_count, path.as_ref().display());

        // Enumerate files
        let mut files = Vec::new();
        for _ in 0 .. file_count
        {
            files.push(FileEnt {
                name: super::CStrBuf::read_from_file(&mut fp, [0; 32])?,
                size: fp.read_u32::<LittleEndian>()?,
                offset: fp.read_u32::<LittleEndian>()?,
                });
        }

        files.sort_by(|a,b| ::std::cmp::Ord::cmp(&*a.name, &*b.name));

        Ok(PodArchive{
            file: fp,
            files: files,
            })
    }

    pub fn open_file<'s>(&'s mut self, path: &str) -> ::std::io::Result<FileHandle<'s>>
    {
        let idx = match self.files.binary_search_by_key(&path.as_bytes(), |v| v.name.as_bytes())
            {
            Ok(i) => i,
            Err(_) => return Err(::std::io::Error::new(::std::io::ErrorKind::NotFound, "")),
            };
        use std::io::Seek;
        self.file.seek(::std::io::SeekFrom::Start(self.files[idx].offset as u64))?;
        Ok(FileHandle {
            file: &mut self.file,
            cur_pos: 0,
            size: self.files[idx].size,
            })
    }
}

pub struct FileHandle<'a>
{
    file: &'a mut ::std::fs::File,
    cur_pos: u32,
    size: u32,
}
impl<'a> ::std::io::Read for FileHandle<'a>
{
    fn read(&mut self, buf: &mut [u8]) -> ::std::io::Result<usize>
    {
        let space = (self.size - self.cur_pos) as usize;

        let buf = if buf.len() > space {
                &mut buf[..space]
            }
            else {
                buf
            };
        let rv = self.file.read(buf)?;
        self.cur_pos += rv as u32;
        Ok(rv)
    }
}