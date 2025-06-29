use async_trait::async_trait;
use std::fmt::Display;
use std::future::Future;
use std::time::Instant;

use crate::output::Output;

#[async_trait]
pub trait TimedFuture: Future + Sized {
    async fn timed<S: AsRef<str> + Display + Send>(
        self,
        title: S,
        output: &Output,
    ) -> Self::Output {
        let start = Instant::now();
        let result = self.await;

        let _ = output.text(format!(
            "{} in {:.2}ms",
            title,
            start.elapsed().as_secs_f64() * 1000.0,
        ));

        result
    }
}

impl<F: Future> TimedFuture for F {}
