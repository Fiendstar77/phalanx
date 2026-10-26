//! Presentation-only, conservative thin-film flow on a sampled height field.
//! This does not change terrain, damage, movement or authoritative simulation RNG.
use phalanx::terrain::Arena;
pub(super) const SIDE: usize = 128;
pub(crate) struct Pools {
    pub height: Vec<f32>,
    volume: Vec<f32>,
    age: Vec<f32>,
    solid: Vec<bool>,
    delta: Vec<f32>,
    half: [f32; 2],
    area: f32,
}
impl Pools {
    pub fn new(arena: Arena) -> Self {
        let half = arena.half_size();
        let mut pool = Self {
            height: vec![0.; SIDE * SIDE],
            volume: vec![0.; SIDE * SIDE],
            age: vec![0.; SIDE * SIDE],
            solid: vec![false; SIDE * SIDE],
            delta: vec![0.; SIDE * SIDE],
            half,
            area: half[0] * half[1] * 4. / (SIDE * SIDE) as f32,
        };
        for z in 0..SIDE {
            for x in 0..SIDE {
                let p = pool.position(x, z);
                let i = z * SIDE + x;
                pool.height[i] = arena.elevation(p);
                pool.solid[i] = arena.blocked(p);
            }
        }
        pool
    }
    pub fn position(&self, x: usize, z: usize) -> [f32; 2] {
        [
            ((x as f32 + 0.5) / SIDE as f32 * 2. - 1.) * self.half[0],
            ((z as f32 + 0.5) / SIDE as f32 * 2. - 1.) * self.half[1],
        ]
    }
    pub fn deposit(&mut self, p: [f32; 2], amount: f32) {
        if amount <= 0.
            || !amount.is_finite()
            || p[0].abs() >= self.half[0]
            || p[1].abs() >= self.half[1]
        {
            return;
        }
        let x = ((p[0] / self.half[0] + 1.) * 0.5 * SIDE as f32) as usize;
        let z = ((p[1] / self.half[1] + 1.) * 0.5 * SIDE as f32) as usize;
        let i = z * SIDE + x;
        if !self.solid[i] {
            self.volume[i] += amount;
            self.age[i] = 0.;
        }
    }
    pub fn depth(&self, i: usize) -> f32 {
        self.volume[i] / self.area
    }
    #[cfg(test)]
    pub fn total(&self) -> f32 {
        self.volume.iter().sum()
    }
    #[cfg(test)]
    pub fn occupied(&self) -> usize {
        self.volume.iter().filter(|v| **v > 0.000_000_1).count()
    }
    pub fn step(&mut self, dt: f32) {
        if dt <= 0. {
            return;
        }
        self.delta.fill(0.);
        for z in 0..SIDE {
            for x in 0..SIDE {
                let i = z * SIDE + x;
                if self.solid[i] {
                    continue;
                }
                for j in [
                    if x + 1 < SIDE { Some(i + 1) } else { None },
                    if z + 1 < SIDE { Some(i + SIDE) } else { None },
                ]
                .into_iter()
                .flatten()
                {
                    if self.solid[j] {
                        continue;
                    }
                    let gradient = self.height[i] + self.depth(i) - self.height[j] - self.depth(j);
                    let (from, to) = if gradient > 0. { (i, j) } else { (j, i) };
                    // Four outgoing edges can never spend more than the source contains.
                    let flux = (gradient.abs() * self.area * dt * 6.).min(self.volume[from] * 0.20);
                    self.delta[from] -= flux;
                    self.delta[to] += flux;
                }
            }
        }
        for i in 0..self.volume.len() {
            self.volume[i] = (self.volume[i] + self.delta[i]).max(0.);
            self.age[i] += dt;
        }
    }
    pub fn texture(&self) -> Vec<u8> {
        // Bilinear texture sampling joins neighbouring cells into a continuous pool.
        let mut pixels = vec![0; SIDE * SIDE * 4];
        for i in 0..SIDE * SIDE {
            let depth = self.depth(i);
            let alpha = (1. - (-depth * 1400.).exp()).clamp(0., 1.);
            let dark = (self.age[i] / 90.).clamp(0., 1.);
            pixels[i * 4] = (87. - dark * 24. - (depth * 1500.).min(20.)) as u8;
            pixels[i * 4 + 1] = 2;
            pixels[i * 4 + 2] = 8;
            pixels[i * 4 + 3] = (alpha * 250.) as u8;
        }
        pixels
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pools_spread_merge_and_conserve_volume_without_entering_walls() {
        let mut p = Pools::new(Arena::OpenField);
        p.height.fill(0.);
        p.solid.fill(false);
        p.deposit([0., 0.], 0.04);
        p.deposit([0.2, 0.], 0.04);
        let initial = p.total();
        let cells = p.occupied();
        for _ in 0..200 {
            p.step(0.05);
        }
        assert!((p.total() - initial).abs() < 0.000_001);
        assert!(p.occupied() > cells * 8);
        let blocked = SIDE / 2 * SIDE + SIDE / 2;
        p.solid[blocked] = true;
        p.volume[blocked] = 0.;
        for _ in 0..20 {
            p.step(0.05);
        }
        assert_eq!(p.volume[blocked], 0.);
        assert!(p.volume.iter().all(|v| v.is_finite() && *v >= 0.));
    }
    #[test]
    fn pooled_blood_flows_downhill_and_pause_preserves_it() {
        let mut a = Pools::new(Arena::OpenField);
        a.solid.fill(false);
        for z in 0..SIDE {
            for x in 0..SIDE {
                a.height[z * SIDE + x] = x as f32 * 0.001;
            }
        }
        a.deposit([0., 0.], 0.04);
        let moment = |p: &Pools| {
            p.volume
                .iter()
                .enumerate()
                .map(|(i, v)| (i % SIDE) as f32 * v)
                .sum::<f32>()
                / p.total()
        };
        let before = moment(&a);
        let frozen = a.volume.clone();
        a.step(0.);
        assert_eq!(a.volume, frozen);
        for _ in 0..100 {
            a.step(0.05);
        }
        assert!(moment(&a) < before - 1.);
        assert!((a.total() - 0.04).abs() < 0.000_001);
    }
}
