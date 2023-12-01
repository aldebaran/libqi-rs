use once_cell::sync::Lazy;
use uuid::Uuid;

#[derive(
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    qi_macros::Reflect,
    qi_macros::FromValue,
    qi_macros::IntoValue,
    qi_macros::ToValue,
)]
#[qi(transparent)]
pub struct MachineId(String);

impl MachineId {
    #[cfg(test)]
    pub(crate) fn new(id: String) -> Self {
        Self(id)
    }

    pub(crate) fn local() -> &'static Self {
        static LOCAL: Lazy<MachineId> = Lazy::new(|| {
            if let Some(id) = MachineId::from_config() {
                return id;
            }

            let mut id = None;
            if cfg!(feature = "machine-uid") {
                id = machine_uid::get().ok().map(Self);
            }
            id.unwrap_or_else(|| {
                let uuid = MachineId::generate_uuid();
                if let Some(path) = MachineId::config_path() {
                    let _res = std::fs::write(path, &uuid);
                }
                Self(uuid)
            })
        });
        &LOCAL
    }

    fn from_config() -> Option<Self> {
        std::fs::read_to_string(Self::config_path()?).ok().map(Self)
    }

    fn config_path() -> Option<std::path::PathBuf> {
        let mut dir = dirs::config_dir()?;
        dir.push("qimessaging");
        dir.push("machine_id");
        Some(dir)
    }

    fn generate_uuid() -> String {
        Uuid::new_v4().to_string()
    }
}
