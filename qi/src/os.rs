use once_cell::sync::Lazy;
use uuid::Uuid;

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct MachineId(Uuid);

impl MachineId {
    pub fn local() -> Self {
        static LOCAL: Lazy<MachineId> = Lazy::new(|| {
            if let Some(id) = MachineId::from_config() {
                return id;
            }

            let mut id = None;
            if cfg!(feature = "machine-uid") {
                id = MachineId::from_machine_uid();
            }
            id.unwrap_or_else(|| {
                let uuid = Uuid::new_v4();
                if let Some(path) = MachineId::config_path() {
                    let _res = std::fs::write(path, uuid);
                }
                Self(uuid)
            })
        });
        *LOCAL
    }

    fn from_config() -> Option<Self> {
        std::fs::read_to_string(Self::config_path()?)
            .ok()?
            .parse()
            .ok()
    }

    #[cfg(feature = "machine-uid")]
    fn from_machine_uid() -> Option<Self> {
        // Custom namespace generated for libqi implementations.
        const QI_NAMESPACE: Uuid = Uuid::from_bytes([
            0xdd, 0x96, 0x97, 0x1d, 0x09, 0x12, 0x44, 0xc2, 0xa4, 0x07, 0x8e, 0x79, 0xa8, 0x29,
            0x7b, 0x89,
        ]);
        Some(Self(Uuid::new_v5(
            &QI_NAMESPACE,
            machine_uid::get().ok()?.as_bytes(),
        )))
    }

    fn config_path() -> Option<std::path::PathBuf> {
        let mut dir = dirs::config_dir()?;
        dir.push("qimessaging");
        dir.push("machine_id");
        Some(dir)
    }

    pub fn as_bytes(&self) -> &uuid::Bytes {
        self.0.as_bytes()
    }
}

impl AsRef<[u8]> for MachineId {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl std::str::FromStr for MachineId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl std::fmt::Display for MachineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl qi_value::Reflect for MachineId {
    fn ty() -> Option<qi_value::Type> {
        Some(qi_value::Type::String)
    }
}

impl qi_value::RuntimeReflect for MachineId {
    fn ty(&self) -> qi_value::Type {
        qi_value::Type::String
    }
}

impl<'a> qi_value::FromValue<'a> for MachineId {
    fn from_value(value: qi_value::Value<'a>) -> Result<Self, qi_value::FromValueError> {
        String::from_value(value)?
            .parse()
            .map_err(Into::into)
            .map_err(qi_value::FromValueError::Other)
    }
}

impl<'a> qi_value::IntoValue<'a> for MachineId {
    fn into_value(self) -> qi_value::Value<'a> {
        qi_value::Value::String(self.to_string().into())
    }
}

impl qi_value::ToValue for MachineId {
    fn to_value(&self) -> qi_value::Value<'_> {
        qi_value::Value::String(self.to_string().into())
    }
}

pub fn process_uuid() -> Uuid {
    static PROCESS_UUID: Lazy<Uuid> = Lazy::new(Uuid::new_v4);
    *PROCESS_UUID
}
