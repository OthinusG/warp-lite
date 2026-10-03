//! Validation and receipt-age rules shared by managed SSH status consumers.
use std::time::{Duration, Instant};

use prost::Message;
use uuid::Uuid;

use crate::proto::{
    cpu_metric, disk_metric, memory_metric, uptime_metric, HostStatus, ManagedFence,
    MetricUnavailable,
};

pub const PROTOCOL_MAJOR: u32 = 1;
pub const MAX_STATUS_BYTES: usize = 8192;
pub const POLL_INTERVAL: Duration = Duration::from_secs(5);
pub const STALE_AFTER: Duration = Duration::from_secs(15);

/// Terminal bytes are bounded and pinned to an admitted process, never local paths.
pub fn valid_terminal_state(state: &crate::proto::TerminalState) -> bool {
    let valid_id = |id: &str| Uuid::parse_str(id).is_ok_and(|id| !id.is_nil());
    state.fence.as_ref().is_some_and(|f| valid_fence(f, true))
        && valid_id(&state.session_id)
        && valid_id(&state.run_id)
        && state.attachment_generation > 0
        && state.output.len() <= 32 * 1024
        && state
            .output_end
            .checked_sub(state.output_offset)
            .is_some_and(|remaining| {
                remaining >= state.output.len() as u64 && remaining <= 256 * 1024
            })
}

/// Nil or malformed IDs never identify a verified attachment.
pub fn valid_fence(fence: &ManagedFence, require_project: bool) -> bool {
    let valid_id = |value: &str| Uuid::parse_str(value).is_ok_and(|id| !id.is_nil());
    valid_id(&fence.service_id)
        && valid_id(&fence.service_boot_id)
        && valid_id(&fence.connection_id)
        && if require_project {
            valid_id(&fence.project_id)
        } else {
            fence.project_id.is_empty()
        }
}

fn valid_reason(reason: i32) -> bool {
    MetricUnavailable::try_from(reason).is_ok_and(|reason| reason != MetricUnavailable::Unspecified)
}

/// All metrics carry a value or a known absence reason; missing is not healthy zero.
pub fn valid_status(status: &HostStatus) -> bool {
    status.encoded_len() <= MAX_STATUS_BYTES
        && status.fence.as_ref().is_some_and(|f| valid_fence(f, true))
        && status.observation_sequence > 0
        && status.sampled_at_unix_millis > 0
        && status.source == "companion_native"
        && matches!(status.os.as_str(), "linux" | "macos" | "windows")
        && matches!(status.architecture.as_str(), "x86_64" | "aarch64")
        && match status.cpu.as_ref().and_then(|m| m.value.as_ref()) {
            Some(cpu_metric::Value::Percent(p)) => p.is_finite() && (0.0..=100.0).contains(p),
            Some(cpu_metric::Value::Unavailable(r)) => valid_reason(*r),
            None => false,
        }
        && match status.memory.as_ref().and_then(|m| m.value.as_ref()) {
            Some(memory_metric::Value::Bytes(b)) => b.total > 0 && b.used <= b.total,
            Some(memory_metric::Value::Unavailable(r)) => valid_reason(*r),
            None => false,
        }
        && match status.project_disk.as_ref().and_then(|m| m.value.as_ref()) {
            Some(disk_metric::Value::Bytes(b)) => b.total > 0 && b.free <= b.total,
            Some(disk_metric::Value::Unavailable(r)) => valid_reason(*r),
            None => false,
        }
        && match status.uptime.as_ref().and_then(|m| m.value.as_ref()) {
            Some(uptime_metric::Value::Seconds(_)) => true,
            Some(uptime_metric::Value::Unavailable(r)) => valid_reason(*r),
            None => false,
        }
}

/// One scope's last observation. Construct a replacement on attachment/project changes.
pub struct HostObservation {
    fence: ManagedFence,
    generation: u64,
    last: Option<(HostStatus, Instant)>,
}

impl HostObservation {
    pub fn new(fence: ManagedFence, generation: u64) -> Option<Self> {
        valid_fence(&fence, true).then_some(Self {
            fence,
            generation,
            last: None,
        })
    }

    /// A rejected callback cannot replace a valid sample or reset its receipt age.
    pub fn receive(&mut self, status: HostStatus, received: Instant) -> bool {
        if !valid_status(&status)
            || status.fence.as_ref() != Some(&self.fence)
            || status.query_generation != self.generation
            || self.last.as_ref().is_some_and(|(last, at)| {
                status.observation_sequence <= last.observation_sequence || received < *at
            })
        {
            return false;
        }
        self.last = Some((status, received));
        true
    }

    pub fn sample(&self) -> Option<&HostStatus> {
        self.last.as_ref().map(|(status, _)| status)
    }

    pub fn age(&self, now: Instant) -> Option<Duration> {
        self.last
            .as_ref()
            .map(|(_, at)| now.saturating_duration_since(*at))
    }

    pub fn stale(&self, now: Instant) -> bool {
        self.age(now).is_none_or(|age| age >= STALE_AFTER)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{CpuMetric, DiskMetric, MemoryBytes, MemoryMetric, UptimeMetric};

    #[test]
    fn status_refuses_invalid_metrics_replaced_scopes_and_old_observations() {
        let fence = ManagedFence {
            service_id: Uuid::new_v4().to_string(),
            service_boot_id: Uuid::new_v4().to_string(),
            connection_id: Uuid::new_v4().to_string(),
            project_id: Uuid::new_v4().to_string(),
        };
        let mut status = HostStatus {
            fence: Some(fence.clone()),
            query_generation: 7,
            observation_sequence: 1,
            sampled_at_unix_millis: 1,
            source: "companion_native".into(),
            os: "linux".into(),
            architecture: "x86_64".into(),
            cpu: Some(CpuMetric {
                value: Some(cpu_metric::Value::Percent(25.0)),
            }),
            memory: Some(MemoryMetric {
                value: Some(memory_metric::Value::Bytes(MemoryBytes {
                    used: 40,
                    total: 100,
                })),
            }),
            project_disk: Some(DiskMetric {
                value: Some(disk_metric::Value::Unavailable(
                    MetricUnavailable::MetricUnsupported.into(),
                )),
            }),
            uptime: Some(UptimeMetric {
                value: Some(uptime_metric::Value::Seconds(10)),
            }),
        };
        let now = Instant::now();
        let mut cache = HostObservation::new(fence, 7).unwrap();
        assert!(cache.stale(now));
        assert!(cache.receive(status.clone(), now));
        assert!(!cache.receive(status.clone(), now + Duration::from_secs(1)));
        assert!(!cache.stale(now + Duration::from_secs(14)));
        assert!(cache.stale(now + STALE_AFTER));
        status.observation_sequence = 2;
        status.query_generation = 8;
        assert!(!cache.receive(status.clone(), now));
        status.query_generation = 7;
        status.fence.as_mut().unwrap().service_boot_id = Uuid::new_v4().to_string();
        assert!(!cache.receive(status.clone(), now));
        status.fence = cache.sample().unwrap().fence.clone();
        for percent in [f32::NAN, f32::INFINITY, -1.0, 100.1] {
            status.cpu.as_mut().unwrap().value = Some(cpu_metric::Value::Percent(percent));
            assert!(!valid_status(&status));
        }
        status.cpu.as_mut().unwrap().value = Some(cpu_metric::Value::Unavailable(
            MetricUnavailable::MetricWarmingUp.into(),
        ));
        assert!(valid_status(&status));
        status.memory.as_mut().unwrap().value = Some(memory_metric::Value::Bytes(MemoryBytes {
            used: 101,
            total: 100,
        }));
        assert!(!valid_status(&status));
        status.memory.as_mut().unwrap().value = Some(memory_metric::Value::Unavailable(0));
        assert!(!valid_status(&status));
        status.memory = None;
        assert!(!valid_status(&status));
        assert_eq!(cache.sample().unwrap().observation_sequence, 1);
    }
}
