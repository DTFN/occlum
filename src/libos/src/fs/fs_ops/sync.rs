use super::*;

pub fn do_sync() -> Result<()> {
    println!("sync:");
    ROOT_INODE.fs().sync()?;
    Ok(())
}
