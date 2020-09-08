use super::*;

pub fn do_chown(path: &str, uid: u32, gid: u32) -> Result<()> {
    println!("chown: path: {:?}, uid: {}, gid: {}", path, uid, gid);
    let inode = {
        let current = current!();
        let fs = current.fs().lock().unwrap();
        fs.lookup_inode(path)?
    };
    let mut info = inode.metadata()?;
    info.uid = uid as usize;
    info.gid = gid as usize;
    inode.set_metadata(&info)?;
    Ok(())
}

pub fn do_fchown(fd: FileDesc, uid: u32, gid: u32) -> Result<()> {
    println!("fchown: fd: {}, uid: {}, gid: {}", fd, uid, gid);
    let file_ref = current!().file(fd)?;
    let mut info = file_ref.metadata()?;
    info.uid = uid as usize;
    info.gid = gid as usize;
    file_ref.set_metadata(&info)?;
    Ok(())
}

pub fn do_lchown(path: &str, uid: u32, gid: u32) -> Result<()> {
    println!("lchown: path: {:?}, uid: {}, gid: {}", path, uid, gid);
    let inode = {
        let current = current!();
        let fs = current.fs().lock().unwrap();
        fs.lookup_inode_no_follow(path)?
    };
    let mut info = inode.metadata()?;
    info.uid = uid as usize;
    info.gid = gid as usize;
    inode.set_metadata(&info)?;
    Ok(())
}
