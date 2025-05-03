use crate::prelude::TestDescAndFuture;
use anyhow::Result;
use flume::Receiver;
use futures::future::try_join_all;
use tokio::task::LocalSet;

#[deprecated = "use tokio::test for now"]
pub async fn run_async(future_rx: Receiver<TestDescAndFuture>) -> Result<()> {
	let mut futs = Vec::new();
	while let Ok(fut) = future_rx.try_recv() {
		futs.push(fut);
	}
	// let future_rx2 = future_rx.clone();
	// let recv_fut = tokio::spawn(async move {
	// 	while let Ok(future) = future_rx2.recv_async().await {
	// 		let a = tokio::spawn(async move {
	// 			let result = (future.fut)().await;
	// 			result_tx.send_async(TestDescAndResult {
	// 				desc: future.desc,
	// 				result:TestResult::Pass,
	// 			})
	// 		});
	// 	}
	// });

	let _fut_result = LocalSet::new()
		.run_until(async move {
			let futs = futs.into_iter().map(|future| {
				tokio::task::spawn_local(async move {
					println!("running future");
					let result = (future.fut)().await;
					println!("Result {}: {:?}", future.desc.name, result);
				})
			});
			try_join_all(futs).await
		})
		.await?;
	Ok(())
}
