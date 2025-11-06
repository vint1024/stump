use std::sync::{
	atomic::{AtomicU32, Ordering},
	Arc,
};
use tokio::sync::Semaphore;

/// Connection pool monitor that provides backpressure for core operations
/// to ensure the API always have access to database connections.
pub struct ConnectionPoolMonitor {
	/// Track active background connections
	connections: Arc<AtomicU32>,
	/// Maximum connections background can use
	max_connections: u32,
	/// Semaphore to limit concurrent background operations
	semaphore: Arc<Semaphore>,
}

impl ConnectionPoolMonitor {
	/// Create a new connection pool monitor.
	///
	/// The monitor will allow background tasks to use up to 80% of the connection pool
	pub fn new(total_pool_size: u32) -> Self {
		// Reserve 20% of pool for API, rest for scanner
		let max_connections = (total_pool_size as f32 * 0.8) as u32;

		tracing::info!(
			total_pool_size,
			max_connections,
			reserved_for_api = total_pool_size - max_connections,
			"Initialized connection pool monitor"
		);

		Self {
			connections: Arc::new(AtomicU32::new(0)),
			max_connections,
			semaphore: Arc::new(Semaphore::new(max_connections as usize)),
		}
	}

	/// Acquire a background connection slot
	///
	/// Returns a guard that will automatically release the slot when dropped.
	pub async fn acquire_slot(&self) -> BackgroundConnectionGuard {
		// Note: The only time I would imagine the semaphore to be closed is during shutdown, but
		// maybe worth handling better with err or optional in the future?
		let permit = self
			.semaphore
			.clone()
			.acquire_owned()
			.await
			.expect("Semaphore should never be closed");

		let current = self.connections.fetch_add(1, Ordering::SeqCst);

		tracing::trace!(
			connections = current + 1,
			max_connections = self.max_connections,
			"Acquired background connection slot"
		);

		BackgroundConnectionGuard {
			_permit: permit,
			counter: self.connections.clone(),
		}
	}

	/// Get current scanner connection count
	pub fn connections_count(&self) -> u32 {
		self.connections.load(Ordering::SeqCst)
	}

	/// Get maximum background connections allowed
	pub fn max_connections(&self) -> u32 {
		self.max_connections
	}

	/// Get number of available background slots
	pub fn available_background_slots(&self) -> u32 {
		self.max_connections
			.saturating_sub(self.connections_count())
	}
}

/// Guard that tracks a background connection slot
///
/// When dropped, the slot is automatically released back to the pool.
pub struct BackgroundConnectionGuard {
	_permit: tokio::sync::OwnedSemaphorePermit,
	counter: Arc<AtomicU32>,
}

impl Drop for BackgroundConnectionGuard {
	fn drop(&mut self) {
		let previous = self.counter.fetch_sub(1, Ordering::SeqCst);
		tracing::trace!(
			connections = previous - 1,
			"Released background connection slot"
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;
	use tokio::time::timeout;

	#[tokio::test]
	async fn test_basic_acquisition() {
		let monitor = ConnectionPoolMonitor::new(50);

		assert_eq!(monitor.connections_count(), 0);
		assert_eq!(monitor.max_connections(), 40); // 80% of 50

		let _guard = monitor.acquire_slot().await;
		assert_eq!(monitor.connections_count(), 1);

		drop(_guard);
		assert_eq!(monitor.connections_count(), 0);
	}

	#[tokio::test]
	async fn test_backpressure() {
		let monitor = ConnectionPoolMonitor::new(5);
		let max_allowed = monitor.max_connections();

		// Loop to acquire max allowed connections
		let mut guards = Vec::new();
		for _ in 0..max_allowed {
			guards.push(monitor.acquire_slot().await);
		}

		assert_eq!(monitor.connections_count(), max_allowed);

		// Try to acquire one more - should block
		let acquire_future = monitor.acquire_slot();
		let result = timeout(Duration::from_millis(100), acquire_future).await;
		assert!(result.is_err(), "Should timeout waiting for slot");

		// Release one slot
		drop(guards.pop());
		assert_eq!(monitor.connections_count(), max_allowed - 1);

		// Now acquiring should work
		let result = timeout(Duration::from_millis(100), monitor.acquire_slot()).await;
		assert!(result.is_ok(), "Should acquire slot after release");
	}

	#[tokio::test]
	async fn test_multiple_releases() {
		let monitor = ConnectionPoolMonitor::new(100);

		let mut guards = Vec::new();
		for _ in 0..10 {
			guards.push(monitor.acquire_slot().await);
		}

		assert_eq!(monitor.connections_count(), 10);

		guards.clear();
		assert_eq!(monitor.connections_count(), 0);
	}
}
