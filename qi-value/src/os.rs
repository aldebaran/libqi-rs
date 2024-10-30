use once_cell::sync::Lazy;
use serde_with::serde_as;
use uuid::Uuid;

use crate::{FromValue, FromValueError, IntoValue, Reflect, RuntimeReflect, ToValue, Type, Value};

#[derive(
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    serde_with::SerializeDisplay,
    serde_with::DeserializeFromStr,
)]
#[serde_as]
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
                MachineId(uuid)
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

impl Reflect for MachineId {
    fn ty() -> Option<Type> {
        Some(Type::String)
    }
}

impl RuntimeReflect for MachineId {
    fn ty(&self) -> Type {
        Type::String
    }
}

impl<'a> FromValue<'a> for MachineId {
    fn from_value(value: Value<'a>) -> Result<Self, FromValueError> {
        String::from_value(value)?
            .parse()
            .map_err(Into::into)
            .map_err(FromValueError::Other)
    }
}

impl<'a> IntoValue<'a> for MachineId {
    fn into_value(self) -> Value<'a> {
        Value::String(self.to_string().into())
    }
}

impl ToValue for MachineId {
    fn to_value(&self) -> Value<'_> {
        Value::String(self.to_string().into())
    }
}

pub fn process_uuid() -> Uuid {
    static PROCESS_UUID: Lazy<Uuid> = Lazy::new(Uuid::new_v4);
    *PROCESS_UUID
}
