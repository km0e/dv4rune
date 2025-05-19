use super::dev::*;
use rune::{alloc, runtime::Object};

#[rune::function(path = cur)]
fn current_user() -> LRes<Object> {
    let mut cfg = Object::default();
    cfg.insert_value(alloc::String::try_from("hid")?, "local")
        .into_result()?;
    cfg.insert_value(alloc::String::try_from("mount")?, "~/.local/share/dv")
        .into_result()?;
    cfg.insert_value(alloc::String::try_from("os")?, "linux")
        .into_result()?;
    Ok(cfg)
}

#[rune::function(path = ssh)]
fn ssh_user(host: &str) -> LRes<Object> {
    let mut cfg = Object::default();
    cfg.insert_value(alloc::String::try_from("host")?, host)
        .into_result()?;
    cfg.insert_value(alloc::String::try_from("mount")?, "~/.local/share/dv")
        .into_result()?;
    cfg.insert_value(alloc::String::try_from("os")?, "linux")
        .into_result()?;
    Ok(cfg)
}

pub fn register(m: &mut rune::module::Module) -> Result<(), rune::ContextError> {
    m.function_meta(current_user)?;
    m.function_meta(ssh_user)?;
    Ok(())
}
