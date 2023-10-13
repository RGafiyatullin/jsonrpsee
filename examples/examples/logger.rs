// Copyright 2019-2021 Parity Technologies (UK) Ltd.
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use std::net::SocketAddr;
use std::time::Instant;

use jsonrpsee::core::{async_trait, client::ClientT};
use jsonrpsee::server::middleware::{Meta, RpcService, RpcServiceT};
use jsonrpsee::server::Server;
use jsonrpsee::types::Request;
use jsonrpsee::ws_client::WsClientBuilder;
use jsonrpsee::{rpc_params, MethodResponse, RpcModule};

#[derive(Clone)]
pub struct Timings(RpcService);

#[async_trait]
impl<'a> RpcServiceT<'a> for Timings {
	async fn call(&self, req: Request<'a>, meta: &Meta) -> MethodResponse {
		let now = Instant::now();
		let name = req.method.to_string();
		let rp = self.0.call(req, meta).await;
		tracing::info!("method call `{name}` took {}ms, metadata: {:?}", now.elapsed().as_millis(), meta);
		rp
	}
}

#[derive(Clone)]
pub struct TimingsLayer;

impl<'a> tower::Layer<RpcService> for TimingsLayer {
	type Service = Timings;

	fn layer(&self, service: RpcService) -> Self::Service {
		Timings(service)
	}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	tracing_subscriber::FmtSubscriber::builder()
		.with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
		.try_init()
		.expect("setting default subscriber failed");

	let addr = run_server().await?;
	let url = format!("ws://{}", addr);

	let client = WsClientBuilder::default().build(&url).await?;
	let _response: String = client.request("say_hello", rpc_params![]).await?;
	let _response: Result<String, _> = client.request("unknown_method", rpc_params![]).await;
	let _: String = client.request("say_hello", rpc_params![]).await?;

	Ok(())
}

async fn run_server() -> anyhow::Result<SocketAddr> {
	let rpc_middleware = tower::ServiceBuilder::new().layer(TimingsLayer);
	let server = Server::builder().set_rpc_middleware(rpc_middleware).build("127.0.0.1:0").await?;
	let mut module = RpcModule::new(());
	module.register_method("say_hello", |_, _| "lo")?;
	let addr = server.local_addr()?;

	let handle = server.start(module);

	// In this example we don't care about doing shutdown so let's it run forever.
	// You may use the `ServerHandle` to shut it down or manage it yourself.
	tokio::spawn(handle.stopped());

	Ok(addr)
}
