mod sessions;

use self::sessions::Sessions;
use crate::{
    object, sd,
    service::{self, ServiceInfo},
    session, Error, MachineId, Object,
};
use bytes::Bytes;
use futures::{future::BoxFuture, FutureExt};
use qi_messaging as messaging;
use std::{future::Future, sync::Arc};

pub fn open() -> (Node, impl Future<Output = ()>) {
    use futures::stream::{FusedStream, FuturesUnordered, StreamExt};
    let (task_sender, mut task_receiver) = tokio::sync::mpsc::channel(1);
    let messaging_service = Arc::new(MessagingService);
    let node = Node {
        messaging_service,
        task_sender,
    };
    let task = async move {
        let mut tasks = FuturesUnordered::new();
        loop {
            tokio::select! {
                Some(task) = task_receiver.recv() => {
                    tasks.push(task);
                }
                Some(()) = tasks.next(), if !tasks.is_terminated() => {
                    // nothing
                }
            }
        }
    };
    (node, task)
}

#[derive(Debug)]
pub struct Node {
    messaging_service: Arc<MessagingService>,
    task_sender: tokio::sync::mpsc::Sender<BoxFuture<'static, ()>>,
}

impl Node {
    pub async fn connect_to_space(self, config: session::Config) -> Result<ClientNode, Error> {
        let (sessions, task) = Sessions::new();
        self.task_sender.send(task.boxed()).await?;
        let session = sessions
            .get(
                config,
                sd::SERVICE_NAME,
                Arc::clone(&self.messaging_service),
            )
            .await?;
        let service_directory = sd::Client::new(session);
        Ok(ClientNode {
            messaging_service: self.messaging_service,
            sessions,
            service_directory,
        })
    }

    fn host_space(self) -> Result<HostNode, Error> {
        todo!()
    }
}

#[derive(Debug)]
pub struct ClientNode {
    messaging_service: Arc<MessagingService>,
    sessions: sessions::Sessions,
    service_directory: sd::Client,
}

impl ClientNode {
    pub fn service_directory(&self) -> &sd::Client {
        &self.service_directory
    }

    pub async fn service(&self, name: &str) -> Result<Box<dyn Object + Send + Sync>, Error> {
        use crate::ServiceDirectory;
        let mut service = self.service_directory.service_info(name).await?;
        sort_service_endpoints(&mut service);
        let session = self
            .sessions
            .get(
                session::Config::default().add_addresses(service.endpoints),
                name,
                Arc::clone(&self.messaging_service),
            )
            .await?;
        let object = object::Client::new(service.service_id, service::MAIN_OBJECT_ID, session);
        Ok(Box::new(object))
    }
}

#[derive(Debug)]
pub struct HostNode;

#[derive(Debug)]
pub(crate) struct MessagingService;

impl messaging::Service for MessagingService {
    fn call(&self, call: messaging::Call) -> BoxFuture<'static, Result<Bytes, messaging::Error>> {
        todo!()
    }

    fn post(&self, post: messaging::Post) -> BoxFuture<'static, Result<(), messaging::Error>> {
        todo!()
    }

    fn event(&self, event: messaging::Event) -> BoxFuture<'static, Result<(), messaging::Error>> {
        todo!()
    }
}

fn sort_service_endpoints(service: &mut ServiceInfo) {
    let service_is_local = &service.machine_id == MachineId::local();
    service.endpoints.sort_by_cached_key(|endpoint| {
        (
            endpoint.is_relative(),
            service_is_local && endpoint.is_loopback(),
        )
    });
}
