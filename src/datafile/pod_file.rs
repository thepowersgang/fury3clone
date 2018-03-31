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
impl ::std::fmt::Debug for FileEnt {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result
    {
        write!(f, "{:?}@{:#x}+{:#x}", &*self.name, self.offset, self.size)
    }
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
    pub fn open_dir_file<'s>(&'s mut self, dir: &str, file: &str) -> ::std::io::Result<FileHandle<'s>>
    {
        debug!("open_dir_file({:?}, {:?})", dir, file);
        let dir = dir.as_bytes();
        let file = file.as_bytes();
        let mut ofs = 0;
        let mut range = 0 .. self.files.len();

        loop
        {
            let b = if ofs < dir.len() {
                    dir[ofs]
                }
                else if ofs == dir.len() {
                    b'\\'
                }
                else if ofs < dir.len() + 1 + file.len() {
                    file[ofs - dir.len() - 1]
                }
                else {
                    break;
                };
            //debug!("ofs = {}, range = {:?}, {:?}", ofs, range, &self.files[range.clone()]);
            let i = match self.files[range.clone()].binary_search_by_key(&b, |v| *v.name.as_bytes().get(ofs).unwrap_or(&255))
                {
                Ok(i) => i,
                Err(_) => {
                    warn!("Not found at +{} '{}' - {:?} {:?} -- {:?}", ofs, b as char, range,
                        &*self.files[range.start].name,
                        &*self.files[range.end-1].name,
                        );
                    return Err(::std::io::Error::new(::std::io::ErrorKind::NotFound, ""))
                    },
                };
            let mut s = range.start + i;
            while s > 0 && s >= range.start && b == *self.files[s].name.as_bytes().get(ofs).unwrap_or(&255) {
                s -= 1;
            }
            if s > 0 {
                s += 1;
            }
            assert_eq!( b, self.files[s].name.as_bytes()[ofs] );
            let mut e = range.start + i;
            while e < range.end && b == *self.files[e].name.as_bytes().get(ofs).unwrap_or(&255) {
                e += 1;
            }
            if e < range.end {
                assert_eq!( b, self.files[e-1].name.as_bytes()[ofs] );
            }

            range = s .. e;
            ofs += 1;
        }
        let idx = range.start;
        if ofs != self.files[idx].name.as_bytes().len() {
            return Err(::std::io::Error::new(::std::io::ErrorKind::NotFound, ""));
        }
        
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
impl<'a> FileHandle<'a>
{
    pub fn size(&self) -> usize {
        self.size as usize
    }
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