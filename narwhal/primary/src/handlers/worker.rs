use crate::proposer::OurDigestMessage;
use anemo::rpc::Status;
use async_trait::async_trait;
use config::{WorkerId, WorkerInfo};
use std::collections::BTreeMap;
use storage::PayloadToken;
use store::Store;
use tokio::sync::oneshot;
use types::metered_channel::Sender;
use types::{BatchDigest, WorkerInfoResponse, WorkerOthersBatchMessage, WorkerOurBatchMessage};

#[async_trait]
pub trait TraitWorkerReceiverController: Sync + Send + 'static {
    async fn report_our_batch(
        &self,
        request: anemo::Request<WorkerOurBatchMessage>,
    ) -> Result<anemo::Response<()>, anemo::rpc::Status> {
        Err(Status::internal("Service not ready"))
    }

    async fn report_others_batch(
        &self,
        request: anemo::Request<WorkerOthersBatchMessage>,
    ) -> Result<anemo::Response<()>, anemo::rpc::Status> {
        Err(Status::internal("Service not ready"))
    }

    async fn worker_info(
        &self,
        _request: anemo::Request<()>,
    ) -> Result<anemo::Response<WorkerInfoResponse>, anemo::rpc::Status> {
        Err(Status::internal("Service not ready"))
    }
}

pub struct UnimplementedWorkerReceiverController {}

impl TraitWorkerReceiverController for UnimplementedWorkerReceiverController {}

#[derive(Clone)]
pub struct WorkerReceiverController {
    pub tx_our_digests: Sender<OurDigestMessage>,
    pub payload_store: Store<(BatchDigest, WorkerId), PayloadToken>,
    pub our_workers: BTreeMap<WorkerId, WorkerInfo>,
}

#[async_trait]
impl TraitWorkerReceiverController for WorkerReceiverController {
    async fn report_our_batch(
        &self,
        request: anemo::Request<WorkerOurBatchMessage>,
    ) -> Result<anemo::Response<()>, anemo::rpc::Status> {
        let message = request.into_body();
        let (tx_ack, rx_ack) = oneshot::channel();
        let response = self
            .tx_our_digests
            .send(OurDigestMessage {
                digest: message.digest,
                worker_id: message.worker_id,
                timestamp: message.metadata.created_at,
                ack_channel: tx_ack,
            })
            .await
            .map(|_| anemo::Response::new(()))
            .map_err(|e| anemo::rpc::Status::internal(e.to_string()))?;

        // If we are ok, then wait for the ack
        rx_ack
            .await
            .map_err(|e| anemo::rpc::Status::internal(e.to_string()))?;

        Ok(response)
    }

    async fn report_others_batch(
        &self,
        request: anemo::Request<WorkerOthersBatchMessage>,
    ) -> Result<anemo::Response<()>, anemo::rpc::Status> {
        let message = request.into_body();
        self.payload_store
            .async_write((message.digest, message.worker_id), 0u8)
            .await;
        Ok(anemo::Response::new(()))
    }

    async fn worker_info(
        &self,
        _request: anemo::Request<()>,
    ) -> Result<anemo::Response<WorkerInfoResponse>, anemo::rpc::Status> {
        Ok(anemo::Response::new(WorkerInfoResponse {
            workers: self.our_workers.clone(),
        }))
    }
}
