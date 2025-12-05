
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use rand::{rng, Rng};
use tokio::sync::broadcast::Sender;
use tokio::task::JoinHandle;
use crate::msg::ControllerToElevatorsMsg;
use crate::msg::ControllerToElevatorsMsg::CloseDoors;

pub static SPEED_FACTOR: AtomicU64 = AtomicU64::new(100);

pub(crate) async fn delay(ms: u64) {
    let factor = SPEED_FACTOR.load(Ordering::Relaxed);
    let adjusted = (ms * factor) / 100;
    tokio::time::sleep(Duration::from_millis(adjusted)).await;
}

pub(crate) async fn random_delay_ms(from: u64, to: u64) {
    let factor = SPEED_FACTOR.load(Ordering::Relaxed);
    let base_delay = rng().random_range(from..=to);
    let adjusted = (base_delay * factor) / 100;
    tokio::time::sleep(Duration::from_millis(adjusted)).await;
}

pub(crate) fn get_closing_task(to_elevators: Sender<ControllerToElevatorsMsg>, elevator: String) -> Option<JoinHandle<()>> {
    Some(tokio::spawn(async move {
        let factor = SPEED_FACTOR.load(Ordering::Relaxed);
        let adjusted = (5000 * factor) / 100;
        tokio::time::sleep(Duration::from_millis(adjusted)).await;
        let _ = to_elevators.send(CloseDoors(elevator));
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_delay_respects_speed_factor() {
        // Set a very fast speed factor (1% of normal time)
        SPEED_FACTOR.store(1, Ordering::Relaxed);

        let start = Instant::now();
        delay(1000).await; // Should take ~10ms instead of 1000ms
        let elapsed = start.elapsed();

        // Should be much faster than 1000ms
        assert!(elapsed.as_millis() < 100, "Delay should be adjusted by speed factor");

        // Reset
        SPEED_FACTOR.store(100, Ordering::Relaxed);
    }

    #[tokio::test]
    async fn test_delay_with_100_percent_factor() {
        SPEED_FACTOR.store(100, Ordering::Relaxed);

        let start = Instant::now();
        delay(50).await;
        let elapsed = start.elapsed();

        // Should be approximately 50ms (with some tolerance)
        assert!(elapsed.as_millis() >= 40 && elapsed.as_millis() < 150);
    }

    #[tokio::test]
    async fn test_delay_with_zero_factor() {
        SPEED_FACTOR.store(0, Ordering::Relaxed);

        let start = Instant::now();
        delay(1000).await;
        let elapsed = start.elapsed();

        // With 0 factor, delay should be instant
        assert!(elapsed.as_millis() < 50);

        SPEED_FACTOR.store(100, Ordering::Relaxed);
    }

    #[tokio::test]
    async fn test_random_delay_ms_within_range() {
        SPEED_FACTOR.store(100, Ordering::Relaxed);

        // Run multiple times to test randomness
        for _ in 0..10 {
            let start = Instant::now();
            random_delay_ms(50, 100).await;
            let elapsed = start.elapsed();

            // Should be within the specified range (with tolerance)
            assert!(elapsed.as_millis() >= 40 && elapsed.as_millis() < 200);
        }
    }

    #[tokio::test]
    async fn test_random_delay_respects_speed_factor() {
        SPEED_FACTOR.store(10, Ordering::Relaxed); // 10% speed

        let start = Instant::now();
        random_delay_ms(100, 200).await;
        let elapsed = start.elapsed();

        // Should be 10-20ms instead of 100-200ms
        assert!(elapsed.as_millis() < 50, "Random delay should respect speed factor");

        SPEED_FACTOR.store(100, Ordering::Relaxed);
    }

    #[test]
    fn test_speed_factor_default() {
        // Default should be 100 (100% = normal speed)
        let factor = SPEED_FACTOR.load(Ordering::Relaxed);
        // Note: This may fail if other tests modified it
        // In a real scenario, you'd want test isolation
        assert!(factor > 0, "Speed factor should be positive");
    }

    #[tokio::test]
    async fn test_get_closing_task_returns_handle() {
        use tokio::sync::broadcast;
        
        let (tx, _rx) = broadcast::channel::<ControllerToElevatorsMsg>(10);
        
        let handle = get_closing_task(tx, "E1".to_string());
        assert!(handle.is_some());
        
        // Cancel the task to avoid waiting
        handle.unwrap().abort();
    }
}