use std::{fs::File, io::BufReader, path::Path};

use liblzma::read::XzDecoder;
use tar::Archive;

use crate::Result;

pub fn unpack_tar_xz<S: AsRef<Path>, D: AsRef<Path>>(
    src: S,
    dst: D,
) -> Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    let file = File::open(src)?;
    let reader = BufReader::new(file);
    let decoder = XzDecoder::new(reader);
    let mut archive = Archive::new(decoder);
    archive.unpack(dst)?;
    Ok(())
}
