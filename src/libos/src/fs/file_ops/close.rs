use super::*;

pub fn do_close(fd: FileDesc) -> Result<()> {
    println!("close: fd: {}", fd);
    let current = current!();
    let mut files = current.files().lock().unwrap();
    files.del(fd)?;
    Ok(())
}
