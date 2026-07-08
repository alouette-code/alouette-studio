use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use crate::memory_inspector::models::TelemetryData;

const DEFAULT_BUFFER_SIZE: usize = 10_000;

/// RingBuffer stores recent telemetry data for fast access and replay.
pub struct TelemetryRingBuffer {
    buffer: VecDeque<TelemetryData>,
    max_size: usize,
}

impl TelemetryRingBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub fn push(&mut self, data: TelemetryData) {
        if self.buffer.len() == self.max_size {
            self.buffer.pop_front();
        }
        self.buffer.push_back(data);
    }

    pub fn get_recent(&self, count: usize) -> Vec<TelemetryData> {
        self.buffer.iter().rev().take(count).cloned().collect()
    }

    pub fn snapshot(&self) -> Vec<TelemetryData> {
        self.buffer.iter().cloned().collect()
    }
}

/// TelemetryPipeline handles asynchronous ingestion of telemetry data
/// ensuring the main event loop is never blocked by metrics collection.
pub struct TelemetryPipeline {
    buffer: Arc<RwLock<TelemetryRingBuffer>>,
    sender: mpsc::Sender<TelemetryData>,
}

impl TelemetryPipeline {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel::<TelemetryData>(1000);
        let buffer = Arc::new(RwLock::new(TelemetryRingBuffer::new(DEFAULT_BUFFER_SIZE)));
        
        let worker_buffer = Arc::clone(&buffer);
        tokio::spawn(async move {
            while let Some(data) = rx.recv().await {
                let mut buf = worker_buffer.write().await;
                buf.push(data);
            }
        });

        Self {
            buffer,
            sender: tx,
        }
    }

    pub async fn ingest(&self, data: TelemetryData) -> Result<(), mpsc::error::SendError<TelemetryData>> {
        self.sender.send(data).await
    }

    pub async fn get_snapshot(&self) -> Vec<TelemetryData> {
        self.buffer.read().await.snapshot()
    }
    
    pub async fn get_recent(&self, count: usize) -> Vec<TelemetryData> {
        self.buffer.read().await.get_recent(count)
    }
}
