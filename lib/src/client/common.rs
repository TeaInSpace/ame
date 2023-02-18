use futures::stream::StreamExt;
use http_body::Body;
use hyper::body::Bytes;
use tokio_stream::Stream;
use tonic::{codegen::StdError, Request};

use crate::{
    grpc::{ame_service_client::AmeServiceClient, ProjectSourceId, ProjectSourceStatus},
    Result,
};

#[derive(Clone)]
pub struct AmeCtrl<C> {
    pub client: AmeServiceClient<C>,
}

impl<'a, C> AmeCtrl<C>
where
    C: tonic::client::GrpcService<tonic::body::BoxBody> + 'a,
    C::Error: Into<StdError>,
    C::ResponseBody: Body<Data = Bytes> + Send + 'static,
    C: Clone,
    <C::ResponseBody as Body>::Error: Into<StdError> + Send,
{
    pub fn new(client: AmeServiceClient<C>) -> AmeCtrl<C> {
        AmeCtrl { client }
    }

    pub async fn watch_project_src(
        self,
        id: ProjectSourceId,
    ) -> Result<impl Stream<Item = ProjectSourceStatus> + 'a> {
        let initial_stream = self
            .client
            .clone()
            .watch_project_src(Request::new(id.clone()))
            .await?
            .into_inner();

        //TODO: ensure this does not turn into an unblocked loop when there is no server available.
        let res = Box::pin(async_stream::stream! {
            let mut stream = initial_stream;
            loop {
                    while let Some(Ok(value)) = stream.next().await {
                        yield value;
                    }

                    stream = self
                .client.clone()
                .watch_project_src(Request::new(id.clone()))
                .await
                .unwrap()
                .into_inner();

                }

        });

        Ok(res)
    }

    pub async fn delete_project_src(mut self, id: ProjectSourceId) -> Result<()> {
        self.client.delete_project_src(Request::new(id)).await?;
        Ok(())
    }
}
