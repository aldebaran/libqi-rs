mod method;

use self::method::BoxMethod;
pub use self::method::Method;
use super::{
    error::{CallError, NoSuchMethodError},
    Object,
};
use async_trait::async_trait;
use qi_value::{
    object::{
        ActionId, MetaMethod, MetaMethodBuilder, MetaMethodBuilderParameter,
        MetaMethodBuilderReturnValue, MetaObject,
    },
    Map, Value,
};
use std::{any::Any, collections::HashMap};

pub struct AnyObject {
    meta: MetaObject,
    instance: Box<dyn Any + Send>,
    methods: HashMap<ActionId, BoxMethod>,
    properties: HashMap<ActionId, Value<'static>>,
}

impl std::fmt::Debug for AnyObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DynamicObject")
    }
}

impl AnyObject {
    pub fn builder() -> ObjectBuilder {
        ObjectBuilder::new()
    }
}

#[async_trait]
impl Object for AnyObject {
    async fn meta_object(&mut self) -> Result<MetaObject, CallError> {
        Ok(self.meta.clone())
    }

    async fn call_with_id(
        &mut self,
        id: ActionId,
        args: Value<'_>,
    ) -> Result<Value<'static>, CallError> {
        let method = self
            .methods
            .get_mut(&id)
            .ok_or_else(|| NoSuchMethodError::Id(id))?;
        let instance = self.instance.as_mut();
        let result = method(instance, args);
        result.await
    }

    async fn property_with_id(&mut self, id: ActionId) -> Option<Value<'static>> {
        self.properties.get(&id).cloned()
    }

    async fn set_property_with_id(&mut self, id: ActionId, value: Value<'_>) -> bool {
        if let Some(property) = self.properties.get_mut(&id) {
            *property = value.into_owned();
            true
        } else {
            false
        }
    }
}

impl super::private::Sealed for AnyObject {}

#[derive(Default)]
pub struct ObjectBuilder {
    methods: Vec<ObjectBuilderMethod>,
    description: String,
}

impl ObjectBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_method<Uid: Into<ActionId>>(&mut self, uid: Uid) -> &mut ObjectBuilderMethod {
        self.methods.push(ObjectBuilderMethod {
            method_builder: MetaMethod::builder(uid),
            method: None,
        });
        self.methods.last_mut().unwrap()
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) -> &mut Self {
        self.description = description.into();
        self
    }

    pub fn build<T>(self, instance: T) -> AnyObject
    where
        T: Any + Send,
    {
        let (meta_methods, methods) = self
            .methods
            .into_iter()
            .map(|method| {
                let uid = method.method_builder.uid();
                (
                    (uid, method.method_builder.build()),
                    (uid, method.method.expect("unbound dynamic object method")),
                )
            })
            .unzip();
        let meta_object = MetaObject {
            methods: meta_methods,
            signals: Map::new(),
            properties: Map::new(),
            description: self.description,
        };
        AnyObject {
            meta: meta_object,
            instance: Box::new(instance),
            methods,
            properties: HashMap::new(),
        }
    }
}

pub struct ObjectBuilderMethod {
    method_builder: MetaMethodBuilder,
    method: Option<BoxMethod>,
}

impl ObjectBuilderMethod {
    pub fn set_name<T: Into<String>>(&mut self, name: T) -> &mut Self {
        self.method_builder.set_name(name);
        self
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) -> &mut Self {
        self.method_builder.set_description(description);
        self
    }

    pub fn return_value(&mut self) -> &mut MetaMethodBuilderReturnValue {
        self.method_builder.return_value()
    }

    pub fn add_parameter(&mut self) -> &mut MetaMethodBuilderParameter {
        self.method_builder.add_parameter()
    }

    pub fn parameter(&mut self, index: usize) -> &mut MetaMethodBuilderParameter {
        self.method_builder.parameter(index)
    }

    pub fn bind<M, Args>(&mut self, method: M) -> &mut Self
    where
        M: Method<Args>,
    {
        self.method_builder
            .return_value()
            .set_type(M::return_type());
        for (idx, parameter_type) in M::parameter_types().into_iter().enumerate() {
            self.method_builder.parameter(idx).set_type(parameter_type);
        }
        self.method = Some(method.boxed());
        self
    }
}
