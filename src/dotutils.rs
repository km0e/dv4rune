use crate::dv::Dv;
use dv_wrap::ops::*;
use rune::support::Result as LRes;
use rune::{
    Any, Mut, Ref,
    runtime::{self, BorrowRef, Object},
};

#[derive(Debug, Default, Clone, Any)]
pub struct ConfigOpt {
    pub name: String,
    #[rune(get, set)]
    pub copy_action: Option<String>,
}

impl ConfigOpt {
    pub fn into_dv(self) -> DotConfig {
        DotConfig {
            name: self.name,
            copy_action: self.copy_action,
        }
    }
}

#[rune::function(free, path = ConfigOpt::new)]
fn config_opt_new(name: String, copy_action: Option<String>) -> ConfigOpt {
    ConfigOpt { name, copy_action }
}

fn object_field<'a, const N: usize>(
    obj: &'a Object,
    field: &[&str; N],
) -> LRes<[BorrowRef<'a, str>; N]> {
    let mut data = [const { std::mem::MaybeUninit::uninit() }; N];
    for (f, v) in field.iter().zip(data.iter_mut()) {
        let Some(f) = obj.get(*f) else {
            return Err(rune::support::Error::msg(format!("field {f} not found")));
        };
        v.write(f.borrow_string_ref()?);
    }
    // Ok(unsafe { std::mem::MaybeUninit::array_assume_init(data) }) //NOTE: this is still in nightly
    Ok(unsafe { std::mem::transmute_copy::<_, [BorrowRef<'a, str>; N]>(&data) })
}

#[derive(Default, Any)]
pub struct RDotUtil {
    inner: DotUtil,
}

#[rune::function(free, path = RDotUtil::new)]
fn dotutil_new(copy_action: Option<String>) -> RDotUtil {
    let inner = DotUtil::new(copy_action);
    RDotUtil { inner }
}

impl RDotUtil {
    #[rune::function(path = Self::add_schema)]
    async fn add_schema(mut this: Mut<Self>, dv: Mut<Dv>, cfg: Object) -> LRes<()> {
        let [user, path] = object_field(&cfg, &["user", "path"])?;
        this.inner
            .add_schema(dv.context(), user.as_ref(), path.as_ref())
            .await?;
        Ok(())
    }
    #[rune::function(path = Self::add_source)]
    async fn add_source(mut this: Mut<Self>, dv: Mut<Dv>, cfg: Object) -> LRes<()> {
        let [user, path] = object_field(&cfg, &["user", "path"])?;
        this.inner
            .add_source(dv.context(), user.as_ref(), path.as_ref())
            .await?;
        Ok(())
    }
    #[rune::function(path = Self::sync)]
    async fn sync(this: Mut<Self>, dv: Mut<Dv>, apps: runtime::Vec, dst: Ref<str>) -> LRes<()> {
        let apps = apps
            .into_iter()
            .map(|value| {
                if let Ok(name) = value.borrow_string_ref() {
                    Ok(DotConfig::new(name.as_ref()))
                } else {
                    rune::from_value::<ConfigOpt>(value).map(|opt| opt.into_dv())
                }
            })
            .collect::<Result<Vec<DotConfig>, runtime::RuntimeError>>()?;
        this.inner
            .sync(dv.context(), apps.clone(), dst.as_ref())
            .await?;
        Ok(())
    }
}
pub fn module() -> Result<rune::Module, rune::ContextError> {
    let mut m = rune::Module::default();
    m.ty::<RDotUtil>()?;
    m.ty::<ConfigOpt>()?;
    m.function_meta(config_opt_new)?;
    m.function_meta(dotutil_new)?;
    m.function_meta(RDotUtil::add_schema)?;
    m.function_meta(RDotUtil::add_source)?;
    m.function_meta(RDotUtil::sync)?;
    Ok(m)
}
