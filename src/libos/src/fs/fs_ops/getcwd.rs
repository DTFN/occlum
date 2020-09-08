use super::*;

pub fn do_getcwd() -> Result<String> {
    println!("getcwd");
    let thread = current!();
    let fs = thread.fs().lock().unwrap();
    let cwd = fs.cwd().to_owned();
    Ok(cwd)
}
