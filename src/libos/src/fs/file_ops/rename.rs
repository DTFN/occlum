use super::*;

pub fn do_rename(oldpath: &str, newpath: &str) -> Result<()> {
    println!("rename: oldpath: {:?}, newpath: {:?}", oldpath, newpath);

    let current = current!();
    let fs = current.fs().lock().unwrap();

    let (old_dir_path, old_file_name) = split_path(&oldpath);
    let (new_dir_path, new_file_name) = split_path(&newpath);
    let old_dir_inode = fs.lookup_inode(old_dir_path)?;
    let new_dir_inode = fs.lookup_inode(new_dir_path)?;
    let old_file_mode = {
        let old_file_inode = old_dir_inode.find(old_file_name)?;
        let metadata = old_file_inode.metadata()?;
        FileMode::from_bits_truncate(metadata.mode)
    };
    if old_file_mode.has_sticky_bit() {
        warn!("ignoring the sticky bit");
    }
    // TODO: support to modify file's absolute path
    old_dir_inode.move_(old_file_name, &new_dir_inode, new_file_name)?;
    Ok(())
}
