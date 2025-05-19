mod pm;
pub use pm::Packages;
mod os;
mod user;

mod dev {
    pub use dv_wrap::ops::*;
    pub use rune::support::Result as LRes;
}

pub fn register(m: &mut rune::module::Module) -> Result<(), rune::ContextError> {
    user::register(m)?;
    os::register(m)?;
    pm::register(m)?;
    Ok(())
}
