use async_trait::async_trait;
use std::fmt::Display;
use std::future::Future;
use std::time::Instant;

#[async_trait]
pub trait TimedFuture: Future + Sized {
    async fn timed<S: AsRef<str> + Display + Send>(self, title: S) -> Self::Output {
        let start = Instant::now();
        let output = self.await;

        println!(
            "{} in {:.2}ms",
            title,
            start.elapsed().as_secs_f64() * 1000.0,
        );

        output
    }
}

impl<F: Future> TimedFuture for F {}
