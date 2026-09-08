//! Wall-clock pacing and asynchronous batch completion. Presentation never chooses
//! how many biological ticks occur. One bounded batch owns its observation epoch.
use crate::*;
use std::sync::mpsc::{self, Receiver, TryRecvError};

pub const SPEED_LABELS: [&str; 9] = ["1x", "2x", "4x", "8x", "16x", "32x", "64x", "128x", "MAX"];
pub const BASE_TPS: u32 = 60;
pub const TELEMETRY_SIZE: u64 = (1 + model::DEATH_STATS_COUNT as u64) * 4;
pub const TIMING_OFFSET: u64 = TELEMETRY_SIZE.next_multiple_of(8);
pub const INSPECTION_OFFSET: u64 = TIMING_OFFSET + 16;
pub const READBACK_SIZE: u64 = INSPECTION_OFFSET
    + (std::mem::size_of::<model::AgentGpu>()
        + std::mem::size_of::<model::PerceptionGpu>()
        + std::mem::size_of::<model::DecisionGpu>()) as u64;

pub struct Scheduler {
    last: Instant,
    speed: usize,
    credit: f64,
    next_allowed: Instant,
    seconds_per_tick: f64,
}
impl Scheduler {
    pub fn new(now: Instant) -> Self {
        Self {
            last: now,
            speed: 0,
            credit: 0.0,
            next_allowed: now,
            seconds_per_tick: 0.00025,
        }
    }
    pub fn reset(&mut self, now: Instant) {
        self.last = now;
        self.credit = 0.0;
        self.next_allowed = now;
    }
    fn goal(&self, speed: usize) -> u32 {
        let responsive = (0.008 / self.seconds_per_tick).floor().clamp(1.0, 32.0) as u32;
        if speed == 8 {
            responsive
        } else {
            responsive.min(1 << speed)
        }
    }
    pub fn take(&mut self, now: Instant, speed: usize) -> u32 {
        if self.speed != speed {
            self.reset(now);
            self.speed = speed;
        }
        let elapsed = now.saturating_duration_since(self.last).as_secs_f64();
        self.last = now;
        if speed != 8 {
            self.credit = (self.credit + elapsed * f64::from(BASE_TPS * (1 << speed))).min(32.0);
        }
        if now < self.next_allowed {
            return 0;
        }
        let goal = self.goal(speed);
        if speed == 8 {
            return goal;
        }
        if self.credit + 1e-6 < f64::from(goal) {
            return 0;
        }
        self.credit -= f64::from(goal);
        goal
    }
    pub fn completed(&mut self, now: Instant, elapsed: Duration, ticks: u32, budget: f32) {
        let seconds = elapsed.as_secs_f64();
        self.seconds_per_tick = self.seconds_per_tick * 0.8 + seconds / f64::from(ticks) * 0.2;
        let idle = seconds * (1.0 / f64::from(budget.clamp(0.1, 1.0)) - 1.0);
        self.next_allowed = now + Duration::from_secs_f64(idle.min(1.0));
    }
    pub fn deadline(&self, now: Instant, speed: usize) -> Instant {
        let credit = if self.speed == speed {
            self.credit
        } else {
            0.0
        };
        let needed = if speed == 8 {
            0.0
        } else {
            (f64::from(self.goal(speed)) - credit).max(0.0) / f64::from(BASE_TPS * (1 << speed))
        };
        (self.last + Duration::from_secs_f64(needed))
            .max(now)
            .max(self.next_allowed)
    }
}

pub struct PendingBatch {
    receiver: Receiver<Result<(), wgpu::BufferAsyncError>>,
    started: Instant,
    ticks: u32,
    inspected: Option<SelectionOutput>,
}

impl AppState {
    pub(crate) fn frame_interval(&self) -> Duration {
        let hz = if self.paused || self.ui.screen != ui::Screen::Play {
            15
        } else {
            self.render_hz
        };
        Duration::from_secs_f64(1.0 / f64::from(hz.max(1)))
    }
    fn running(&self) -> bool {
        self.ui.screen == ui::Screen::Play && (!self.paused || self.step_requested)
    }
    pub(crate) fn next_wake(&self, now: Instant) -> Instant {
        let frame = if self.occluded {
            now + Duration::from_millis(100)
        } else {
            self.next_frame.max(now + Duration::from_millis(1))
        };
        let sim = if self.pending_batch.is_some() {
            now + Duration::from_millis(1)
        } else if self.running() {
            self.scheduler
                .deadline(now, self.speed_index)
                .max(now + Duration::from_micros(100))
        } else {
            frame
        };
        frame.min(sim)
    }
    pub(crate) fn pump_simulation(&mut self) {
        self.complete_batch(false);
        let now = Instant::now();
        if !self.running() {
            self.scheduler.reset(now);
            return;
        }
        if self.pending_batch.is_some() {
            return;
        }
        if self.living_agents == 0 {
            self.step_requested = false;
            self.service_completed_world();
            return;
        }
        let mut ticks = if self.step_requested {
            self.step_requested = false;
            1
        } else {
            self.scheduler.take(now, self.speed_index)
        }
        .min(model::MAX_WORLD_TICKS.saturating_sub(self.simulation.tick));
        if let Some(until_win) = self.simulation.ticks_until_challenger_can_win() {
            ticks = ticks.min(until_win);
        }

        if self.simulation.tick >= model::MAX_WORLD_TICKS {
            self.paused = true;
            self.file_status =
                "World tick capacity reached. Save is available; this is not an extinction.".into();
            return;
        }

        if ticks == 0 {
            return;
        }
        let mut e = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("independent simulation batch"),
            });
        if let Some(timing) = &self.gpu_timing {
            e.write_timestamp(&timing.query_set, 0);
        }
        self.simulation
            .encode_ticks(&mut e, &self.device, &self.queue, ticks);
        if let Some(timing) = &self.gpu_timing {
            e.write_timestamp(&timing.query_set, 1);
            e.resolve_query_set(&timing.query_set, 0..2, &timing.resolve_buffer, 0);
            e.copy_buffer_to_buffer(
                &timing.resolve_buffer,
                0,
                &self.batch_readback,
                TIMING_OFFSET,
                16,
            );
        }
        self.simulation
            .encode_telemetry(&mut e, &self.batch_readback);
        let inspected = if self.inspection.following
            && now.duration_since(self.last_inspection) >= Duration::from_millis(100)
        {
            self.last_inspection = now;
            self.inspection
                .snapshot
                .filter(|s| s.selected > 0 && s.selected <= MAX_AGENTS)
        } else {
            None
        };
        if let Some(previous) = inspected {
            let slot = u64::from(previous.selected - 1);
            let mut offset = INSPECTION_OFFSET;
            for (source, size) in [
                (
                    &self.simulation.agent_buffers[self.simulation.current_buffer],
                    std::mem::size_of::<model::AgentGpu>() as u64,
                ),
                (
                    &self.simulation.perception_buffer,
                    std::mem::size_of::<model::PerceptionGpu>() as u64,
                ),
                (
                    &self.simulation.decision_buffer,
                    std::mem::size_of::<model::DecisionGpu>() as u64,
                ),
            ] {
                e.copy_buffer_to_buffer(source, slot * size, &self.batch_readback, offset, size);
                offset += size;
            }
        }
        self.queue.submit(Some(e.finish()));
        let (tx, rx) = mpsc::channel();
        self.batch_readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        self.pending_batch = Some(PendingBatch {
            receiver: rx,
            started: now,
            ticks,
            inspected,
        });
    }

    /// Only explicit user actions/save/close may request a blocking drain.
    pub(crate) fn complete_batch(&mut self, wait: bool) {
        if self.pending_batch.is_none() {
            return;
        }
        self.device.poll(if wait {
            wgpu::Maintain::Wait
        } else {
            wgpu::Maintain::Poll
        });
        let result = self.pending_batch.as_ref().unwrap().receiver.try_recv();
        match result {
            Err(TryRecvError::Empty) => return,
            Ok(Ok(())) => {}
            _ => {
                self.pending_batch = None;
                self.batch_readback.unmap();
                self.paused = true;
                self.file_status =
                    "Simulation readback failed; paused to preserve the world".into();
                return;
            }
        }
        let pending = self.pending_batch.take().unwrap();
        let mapped = self.batch_readback.slice(..).get_mapped_range();
        let u32_at =
            |offset: usize| bytemuck::pod_read_unaligned::<u32>(&mapped[offset..offset + 4]);
        self.living_agents = u32_at(0);
        self.food_eaten = u64::from(u32_at(4)) + (u64::from(u32_at(60)) << 32);
        self.starvation_deaths = u32_at(8);
        self.age_deaths = u32_at(12);
        self.births = u32_at(16);
        self.interaction_stats = [u32_at(20), u32_at(24), u32_at(28), u32_at(32)];
        if let Some(timing) = &self.gpu_timing {
            let start = bytemuck::pod_read_unaligned::<u64>(
                &mapped[TIMING_OFFSET as usize..TIMING_OFFSET as usize + 8],
            );
            let end = bytemuck::pod_read_unaligned::<u64>(
                &mapped[TIMING_OFFSET as usize + 8..INSPECTION_OFFSET as usize],
            );
            self.gpu_tick_ms = Some(
                end.saturating_sub(start) as f32 * timing.timestamp_period_ns
                    / 1_000_000.0
                    / pending.ticks as f32,
            );
        }
        if let Some(previous) = pending.inspected {
            let mut offset = INSPECTION_OFFSET as usize;
            let mut current = previous;
            let size = std::mem::size_of::<model::AgentGpu>();
            current.agent = bytemuck::pod_read_unaligned(&mapped[offset..offset + size]);
            offset += size;
            let size = std::mem::size_of::<model::PerceptionGpu>();
            current.perception = bytemuck::pod_read_unaligned(&mapped[offset..offset + size]);
            offset += size;
            let size = std::mem::size_of::<model::DecisionGpu>();
            current.decision = bytemuck::pod_read_unaligned(&mapped[offset..offset + size]);
            let same = current.agent.generation == previous.agent.generation
                && current.agent.lineage_id == previous.agent.lineage_id
                && current.agent.birth_tick == previous.agent.birth_tick;
            self.inspection
                .refresh(Ok(same.then_some(current)), self.simulation.tick);
        }
        drop(mapped);
        self.batch_readback.unmap();
        self.update_selection_highlight();
        let now = Instant::now();
        self.scheduler.completed(
            now,
            now.duration_since(pending.started),
            pending.ticks,
            self.compute_budget,
        );
        self.ticks_window_accumulated = self.ticks_window_accumulated.saturating_add(pending.ticks);
        self.world_revision = self.world_revision.saturating_add(1);

        if let Some(experiment) = &mut self.experiment {
            experiment.total_ticks = experiment.total_ticks.saturating_add(pending.ticks as u64);
        }
        if now.duration_since(self.last_metrics) >= Duration::from_secs(1) {
            self.last_metrics = now;
            if let Ok(metrics) = self.simulation.metrics(&self.device, &self.queue) {
                if self.history.len() >= 400 {
                    self.history.pop_front();
                }
                self.history.push_back(metrics);
            }
        }
        self.service_completed_world();
    }

    fn service_completed_world(&mut self) {
        if self.living_agents != 0 {
            match self
                .simulation
                .promote_challenger_if_outlived(u64::from(self.living_agents))
            {
                Ok(true) => {
                    self.world_revision = self.world_revision.saturating_add(1);
                    self.file_status =
                        "Candidate outlived its incumbent and is now the live population.".into();
                }
                Ok(false) => {}
                Err(e) => {
                    self.paused = true;
                    self.file_status = format!("Evolution paused: {e}");
                    return;
                }
            }
        }
        if self.living_agents == 0 && self.simulation.progress.completed.is_none() {
            match self.simulation.complete_world(&self.device, &self.queue) {
                Ok(()) => self.world_revision = self.world_revision.saturating_add(1),
                Err(e) => {
                    self.paused = true;
                    self.file_status = format!("Population comparison paused: {e}");
                    return;
                }
            }
        }
        if self.living_agents == 0 && !self.paused {
            let result = (|| -> Result<(), String> {
                self.simulation.advance_world(&self.device, &self.queue)?;
                self.world_revision = self.world_revision.saturating_add(1);
                self.clear_world_observers();
                self.refresh_metrics()?;
                self.file_status = format!(
                    "World {}: {:?} population evaluation",
                    self.simulation.progress.world, self.simulation.progress.phase
                );
                Ok(())
            })();
            if let Err(e) = result {
                self.paused = true;
                self.file_status = format!("Evolution paused: {e}");
            }
        }
        if self.experiment.is_some()
            && self.ui.screen == ui::Screen::Play
            && self.last_autosave.elapsed() >= Duration::from_secs(300)
        {
            self.file_status = match self.save_experiment() {
                Ok(m) => m,
                Err(e) => {
                    self.paused = true;
                    format!("Autosave failed; paused: {e}")
                }
            };
            self.last_autosave = Instant::now();
        }
    }
}
