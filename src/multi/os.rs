use os2::Os;
use rune::Ref;

#[rune::function(instance)]
fn compat(this: Ref<Os>, os: &str) -> bool {
    let os = Os::from(os);
    this.compatible(&os)
}

#[rune::function(instance)]
fn as_str(this: Ref<Os>) -> String {
    this.to_string()
}
pub fn register(m: &mut rune::module::Module) -> Result<(), rune::ContextError> {
    m.ty::<Os>()?;
    m.function_meta(compat)?;
    m.function_meta(as_str)?;
    Ok(())
}
