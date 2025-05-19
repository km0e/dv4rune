use std::{collections::HashMap, future::IntoFuture, path::Path};

use dv_api::{
    fs::{CheckInfo, Metadata, OpenFlags, U8Path},
    process::Interactor,
    whatever,
};
use dv_wrap::{Context, DeviceInfo, SqliteCache, TermInteractor, User, ops::Pm};
use os2::Os;
use rune::{
    Any, from_value,
    runtime::{self, Mut, Ref},
    support,
};

use support::Result as LRes;

use crate::multi::Packages;

#[derive(Debug)]
struct Device {
    pub info: DeviceInfo,
    system: Option<String>,
    users: Vec<String>,
}

impl Device {
    pub fn new(info: DeviceInfo) -> Self {
        Self {
            info,
            system: None,
            users: Vec::new(),
        }
    }
}

#[derive(Any)]
pub struct Dv {
    dry_run: bool,
    devices: HashMap<String, Device>,
    users: HashMap<String, User>,
    cache: SqliteCache,
    interactor: TermInteractor,
}

impl Dv {
    pub fn new(path: impl AsRef<Path>, dry_run: bool) -> Self {
        Self {
            dry_run,
            devices: HashMap::new(),
            users: HashMap::new(),
            cache: SqliteCache::new(path),
            interactor: TermInteractor::new().unwrap(),
        }
    }
    pub fn context(&self) -> Context<'_> {
        Context::new(self.dry_run, &self.cache, &self.interactor, &self.users)
    }
}
impl Dv {
    #[rune::function(path = Self::copy)]
    async fn copy(
        this: Ref<Self>,
        src: (Ref<str>, Ref<str>),
        dst: (Ref<str>, Ref<str>),
        confirm: Option<Ref<str>>,
    ) -> LRes<bool> {
        Ok(
            dv_wrap::ops::CopyContext::new(this.context(), &src.0, &dst.0, confirm.as_deref())?
                .copy(src.1, dst.1)
                .await?,
        )
    }
    #[rune::function(path = Self::exec)]
    async fn exec(
        this: Ref<Self>,
        uid: Ref<str>,
        shell: Option<Ref<str>>,
        commands: Ref<str>,
    ) -> LRes<bool> {
        Ok(dv_wrap::ops::exec(&this.context(), uid, shell.as_deref(), commands).await?)
    }
    #[rune::function(path = Self::auto)]
    async fn auto(
        this: Ref<Self>,
        uid: Ref<str>,
        service: Ref<str>,
        action: Ref<str>,
        args: Option<Ref<str>>,
    ) -> LRes<bool> {
        let args = args.as_ref().map(|s| s.as_ref());
        Ok(dv_wrap::ops::auto(&this.context(), uid, service, action, args).await?)
    }
    #[rune::function(path = Self::once)]
    async fn once(
        this: Ref<Self>,
        id: Ref<str>,
        key: Ref<str>,
        f: runtime::Function,
    ) -> LRes<bool> {
        let once = dv_wrap::ops::Once::new(this.context(), id.as_ref(), key.as_ref());
        let res = once.test().await?//not cached
        && {
            let res: LRes<bool> = rune::from_value(
                f.call::<runtime::Future>(())
                    .into_result()?
                    .into_future()
                    .await
                    .into_result()?,
            )?;
            let res= res?;
            once.set().await?;
            res
        };
        Ok(res)
    }
    #[rune::function(path = Self::refresh)]
    async fn refresh(this: Ref<Self>, id: Ref<str>, key: Ref<str>) -> LRes<()> {
        dv_wrap::ops::refresh(this.context(), id, key).await?;
        Ok(())
    }
    #[rune::function(path = Self::load_src)]
    async fn load_src(this: Ref<Self>, id: Ref<str>, path: Ref<str>) -> LRes<runtime::Vec> {
        let id = id.as_ref();
        if let Some(user) = this.users.get(id) {
            let path = path.as_ref();
            let res = user.check_path(path).await?;
            let mut srcs = runtime::Vec::new();
            let copy = async |src: &U8Path| -> LRes<runtime::Value> {
                let mut src = user.open(src, OpenFlags::READ).await?;
                let dst = tempfile::NamedTempFile::new()?;
                let (file, path) = dst.keep()?;
                let mut file = tokio::fs::File::from_std(file);
                tokio::io::copy(&mut src, &mut file).await?;
                Ok(rune::to_value(path.to_string_lossy().to_string())?)
            };
            match res {
                CheckInfo::File(f) => {
                    srcs.push(copy(&f.path).await?)?;
                }
                CheckInfo::Dir(di) => {
                    let mut buf = di.path.clone();
                    for Metadata { path, .. } in di.files {
                        buf.push(&path);
                        srcs.push(copy(&buf).await?)?;
                        buf.clone_from(&di.path);
                    }
                }
            }
            Ok(srcs)
        } else {
            Err(rune::support::Error::msg("missing user"))
        }
    }
}

impl Dv {
    #[rune::function(path = Self::add_user)]
    async fn add_user(mut this: Mut<Dv>, id: Ref<str>, obj: runtime::Object) -> LRes<()> {
        let uid = id.to_string();
        if this.users.contains_key(&uid) {
            whatever!("user {} already exists", uid);
        }
        let mut cfg = dv_api::multi::Config::default();
        for (name, value) in obj {
            if name == "is_system" {
                cfg.is_system = Some(value.as_bool()?);
                continue;
            }
            cfg.set(name.to_string(), from_value::<String>(value)?);
        }
        let hid = cfg.get("hid").cloned();
        let u = User::new(cfg).await?; //NOTE:use device's os init?
        if let Some(hid) = hid {
            let hid = hid.to_string();
            let dev = match this.devices.get_mut(&hid) {
                Some(dev) => dev,
                None => {
                    let dev = Device::new(DeviceInfo::detect(&u, u.os()).await?);
                    this.devices.insert(hid.clone(), dev);
                    this.devices.get_mut(&hid).unwrap()
                }
            };
            if u.is_system {
                dev.system = Some(uid);
            } else {
                dev.users.push(uid);
            }
        };
        this.interactor
            .log(format!("user: {:<10}, os: {:<8}", id.as_ref(), u.os()))
            .await;
        this.users.insert(id.to_string(), u);
        Ok(())
    }
}

impl Dv {
    #[rune::function(path = Self::pm)]
    async fn pm(this: Ref<Self>, uid: Ref<str>, packages: Packages) -> LRes<bool> {
        let uid = uid.as_ref();
        let user = this.context().get_user(uid)?;
        let pm = match this.devices.get(uid).map(|dev| dev.info.pm) {
            Some(pm) => pm,
            None => Pm::detect(user, &user.os()).await?,
        };
        Ok(packages
            .as_package()
            .install(this.context(), uid, &pm)
            .await?)
    }
}

impl Dv {
    #[rune::function(path = Self::os)]
    fn os(this: Ref<Self>, uid: Ref<str>) -> LRes<Os> {
        let uid = uid.as_ref();
        let user = this.context().get_user(uid)?;
        Ok(user.os())
    }
}

pub fn module() -> Result<rune::Module, rune::ContextError> {
    let mut m = rune::Module::with_crate("dv")?;
    m.ty::<Dv>()?;
    crate::multi::register(&mut m)?;
    m.function_meta(Dv::add_user)?;
    m.function_meta(Dv::auto)?;
    m.function_meta(Dv::copy)?;
    m.function_meta(Dv::exec)?;
    m.function_meta(Dv::load_src)?;
    m.function_meta(Dv::once)?;
    m.function_meta(Dv::os)?;
    m.function_meta(Dv::pm)?;
    m.function_meta(Dv::refresh)?;
    Ok(m)
}
