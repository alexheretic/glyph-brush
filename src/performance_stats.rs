//! Performance logging logic, enabled with feature `performance_stats` and info level logging
use std::mem;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) struct PerformanceStats {
    partial: Vec<Instant>,
    layout_calls: Vec<Call>,
    draw: Option<DrawCall>,
}

impl PerformanceStats {
    pub(crate) fn draw_start(&mut self) {
        let t = Instant::now();
        self.partial.clear();
        self.partial.push(t);
    }

    pub(crate) fn gpu_cache_done(&mut self) {
        let t = Instant::now();
        assert!(self.partial.len() == 1, "{:?}", self.partial);
        self.partial.push(t);
    }

    pub(crate) fn vertex_generation_done(&mut self) {
        let t = Instant::now();
        assert!(self.partial.len() == 2, "{:?}", self.partial);
        self.partial.push(t);
    }

    pub(crate) fn draw_finished(&mut self) {
        let t = Instant::now();
        if self.partial.len() == 3 {
            // had to re-gen
            let vertex_done = self.partial.pop().unwrap();
            let gpu_cache_done = self.partial.pop().unwrap();
            let start = self.partial.pop().unwrap();
            self.draw = Some(DrawCall {
                start,
                gpu_cache_done,
                vertex_done,
                all_done: t,
            });
        } else if self.partial.len() == 1 {
            // cached
            let start = self.partial.pop().unwrap();
            self.draw = Some(DrawCall {
                start,
                gpu_cache_done: start,
                vertex_done: start,
                all_done: t,
            });
        } else {
            panic!("Incorrect partial {:?}", self.partial);
        }
    }

    pub(crate) fn layout_start(&mut self) {
        let t = Instant::now();
        self.partial.clear();
        self.partial.push(t);
    }

    pub(crate) fn layout_finished(&mut self) {
        let t = Instant::now();
        assert!(self.partial.len() == 1);
        self.layout_calls.push(Call(self.partial.remove(0), t));
    }

    pub(crate) fn log_sluggishness(&mut self) {
        let mut draw = self.draw.take();
        let mut layout_calls = vec![];
        mem::swap(&mut self.layout_calls, &mut layout_calls);

        if draw.is_none() {
            return;
        }
        let draw = draw.take().unwrap();

        let layout_cost: Duration = layout_calls
            .iter()
            .map(|&Call(start, end)| end - start)
            .sum();
        let draw_cost = draw.all_done - draw.start;
        if draw_cost + layout_cost < Duration::new(0, 1_000_000) {
            return;
        }

        info!(
            "Total {total:.1}ms, \
             layout ({nlayout}x) {layout:.1}ms, \
             draw {draw:.1}ms (\
             gpu-cache {gpu:.1}ms, \
             vertex-gen {vert:.1}ms, \
             draw-call {draw_call:.1}ms)",
            total = f64::from((draw_cost + layout_cost).subsec_nanos()) / 1_000_000_f64,
            nlayout = layout_calls.len(),
            layout = f64::from(layout_cost.subsec_nanos()) / 1_000_000_f64,
            draw = f64::from(draw_cost.subsec_nanos()) / 1_000_000_f64,
            gpu = f64::from((draw.gpu_cache_done - draw.start).subsec_nanos()) / 1_000_000_f64,
            vert =
                f64::from((draw.vertex_done - draw.gpu_cache_done).subsec_nanos()) / 1_000_000_f64,
            draw_call =
                f64::from((draw.all_done - draw.vertex_done).subsec_nanos()) / 1_000_000_f64,
        );
    }
}

#[derive(Debug, Clone, Copy)]
struct Call(Instant, Instant);

#[derive(Debug, Clone, Copy)]
struct DrawCall {
    start: Instant,
    gpu_cache_done: Instant,
    vertex_done: Instant,
    all_done: Instant,
}
