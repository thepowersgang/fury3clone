//!
//! 
//!

pub use self::pod_file::PodArchive;
pub use self::pod_file::FileHandle;
pub use self::model::Model;

mod pod_file;

mod model;

struct CStrBuf<A>
{
    buf: A,
}
impl<A> CStrBuf<A>
where
    A: AsRef<[u8]>
{
    fn new(buf: A) -> CStrBuf<A>
    {
        assert!( buf.as_ref().iter().position(|&v| v == 0).is_some() );
        CStrBuf {
            buf: buf,
            }
    }

    fn read_from_file<F: ::std::io::Read>(fp: &mut F, mut buf: A) -> ::std::io::Result<Self>
    where
        A: AsMut<[u8]>
    {
        fp.read(buf.as_mut())?;
        Ok( CStrBuf::new(buf) )
    }

    fn as_bytes(&self) -> &[u8]
    {
        let src = self.as_bytes_with_nul();
        assert_eq!(src[src.len()-1], 0);
        let rv = &src[.. src.len()-1];
        for v in rv {
            assert!(*v != 0);
        }
        rv
    }
    fn as_bytes_with_nul(&self) -> &[u8]
    {
        let len = self.buf.as_ref().iter().position(|&v| v == 0).unwrap();
        &self.buf.as_ref()[..len+1]
    }
}
impl<A> ::std::ops::Deref for CStrBuf<A>
where
    A: AsRef<[u8]>
{
    type Target = ::std::ffi::CStr;
    fn deref(&self) -> &Self::Target
    {
        let len = self.buf.as_ref().iter().position(|&v| v == 0).unwrap();
        ::std::ffi::CStr::from_bytes_with_nul(&self.buf.as_ref()[..len+1]).unwrap()
    }
}