use once_cell::sync::Lazy;
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, qi_macros::Valuable)]
#[qi(value = "qi_value", transparent)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use qi_format::de::BufExt;

    #[test]
    fn test_deserialize() {
        let mut input = &[
            0x24, 0x00, 0x00, 0x00, 0x39, 0x61, 0x36, 0x35, 0x62, 0x35, 0x36, 0x65, 0x2d, 0x63,
            0x33, 0x64, 0x33, 0x2d, 0x34, 0x34, 0x38, 0x35, 0x2d, 0x38, 0x39, 0x32, 0x34, 0x2d,
            0x36, 0x36, 0x31, 0x62, 0x30, 0x33, 0x36, 0x32, 0x30, 0x32, 0x62, 0x33,
        ][..];
        let machine_id: MachineId = input.deserialize_value().unwrap();
        assert_eq!(
            machine_id,
            MachineId::new("9a65b56e-c3d3-4485-8924-661b036202b3".to_owned()),
        )
    }
}
