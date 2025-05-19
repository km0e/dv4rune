use std::collections::HashMap;

use super::dev::*;

#[derive(Debug, Default, rune::Any)]
pub struct Packages {
    pm: HashMap<Pm, String>,
}

impl std::fmt::Display for Packages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.pm.is_empty() {
            write!(f, "empty")
        } else {
            for (pm, package) in &self.pm {
                write!(f, "{}:{} ", pm, package)?;
            }
            Ok(())
        }
    }
}

impl Packages {
    pub fn as_package(&self) -> Package {
        Package {
            pm: self.pm.iter().map(|(k, v)| (*k, v.as_str())).collect(),
        }
    }

    #[rune::function(path = Self::new)]
    pub fn new() -> Packages {
        Packages::default()
    }

    #[rune::function(path = Self::add_assign, protocol=ADD_ASSIGN)]
    fn add_assign(this: &mut Packages, value: &Packages) {
        for (k, v) in &value.pm {
            let entry = this.pm.entry(*k).or_default();
            entry.push(' ');
            entry.push_str(v);
        }
    }

    #[rune::function(path = Self::index_set, protocol = INDEX_SET)]
    pub fn index_set(&mut self, key: &str, value: &str) -> LRes<()> {
        self.pm.insert(key.parse()?, value.to_string());
        Ok(())
    }
}

pub fn register(m: &mut rune::module::Module) -> Result<(), rune::ContextError> {
    m.ty::<Packages>()?;
    m.function_meta(Packages::new)?;
    m.function_meta(Packages::add_assign)?;
    m.function_meta(Packages::index_set)?;
    Ok(())
}
